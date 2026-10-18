// Settings — edit and validate the persisted monitor configuration.
import { useEffect, useState } from 'react'
import type { ReactNode } from 'react'
import { Badge, ErrorBanner, Loading, MetricRow, Panel } from '../components/ui'
import type { Tone } from '../components/ui'
import { useSettings } from '../hooks/useMonitor'
import { describeError, pruneHistory } from '../lib/api'
import { DEFAULT_CONFIG, PLACEHOLDER } from '../types'
import type { AppConfig, ConnectionTest } from '../types'

const SERVER_INTERVAL_MIN = 250
const SLOTS_INTERVAL_MIN = 250
const HARDWARE_INTERVAL_MIN = 250
const STORAGE_INTERVAL_MIN = 1000
const RETENTION_MIN_HOURS = 1

/** A single editable profile row. */
interface ProfileValues {
  name: string
  serverUrl: string
}

/** Editable form state; numbers are held as strings so invalid input survives. */
interface FormValues {
  serverUrl: string
  profiles: ProfileValues[]
  activeProfile: string
  serverIntervalMs: string
  slotsIntervalMs: string
  hardwareIntervalMs: string
  storageIntervalMs: string
  retentionHours: string
}

type FieldErrors = Partial<Record<keyof FormValues, string>>

/** Projects a config into editable form strings. */
function toForm(c: AppConfig): FormValues {
  return {
    serverUrl: c.serverUrl,
    profiles: c.profiles.map((p) => ({ name: p.name, serverUrl: p.serverUrl })),
    activeProfile: c.activeProfile,
    serverIntervalMs: String(c.serverIntervalMs),
    slotsIntervalMs: String(c.slotsIntervalMs),
    hardwareIntervalMs: String(c.hardwareIntervalMs),
    storageIntervalMs: String(c.storageIntervalMs),
    retentionHours: String(c.retentionHours),
  }
}

/** Parses a whole number that must be at least `min`. */
function parseNumber(raw: string, min: number, minLabel: string): { value: number | null; error: string | null } {
  const trimmed = raw.trim()
  if (trimmed === '') return { value: null, error: 'Required' }
  const n = Number(trimmed)
  if (!Number.isFinite(n)) return { value: null, error: 'Enter a number' }
  if (!Number.isInteger(n)) return { value: null, error: 'Must be a whole number' }
  if (n < min) return { value: null, error: `Must be at least ${minLabel}` }
  return { value: n, error: null }
}

/** Validates the form and, when clean, returns a saveable config. */
function validate(v: FormValues): { errors: FieldErrors; config: AppConfig | null } {
  const errors: FieldErrors = {}

  // --- Server URL (effective URL of the active profile) ---
  const url = v.serverUrl.trim()
  if (url === '') {
    errors.serverUrl = 'Required'
  } else {
    try {
      const parsed = new URL(url)
      if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') {
        errors.serverUrl = 'URL must use http or https'
      }
    } catch {
      errors.serverUrl = 'Enter a valid URL'
    }
  }

  // --- Profiles ---
  let profileError: string | null = null
  if (v.profiles.length === 0) {
    profileError = 'At least one profile is required'
  } else {
    const names = v.profiles.map((p) => p.name.trim())
    if (names.some((n) => n === '')) {
      profileError = 'Profile name is required'
    } else if (new Set(names).size !== names.length) {
      profileError = 'Profile names must be unique'
    } else {
      const badUrl = v.profiles.find((p) => {
        try {
          const parsed = new URL(p.serverUrl.trim())
          return parsed.protocol !== 'http:' && parsed.protocol !== 'https:'
        } catch {
          return true
        }
      })
      if (badUrl) profileError = 'Enter a valid URL'
    }
  }
  if (profileError) errors.profiles = profileError

  // --- Active profile ---
  let activeError: string | null = null
  if (v.activeProfile.trim() === '') {
    activeError = 'Required'
  } else if (!v.profiles.some((p) => p.name === v.activeProfile.trim())) {
    activeError = 'Unknown profile'
  }
  if (activeError) errors.activeProfile = activeError

  // --- Intervals (unchanged) ---
  const server = parseNumber(v.serverIntervalMs, SERVER_INTERVAL_MIN, `${SERVER_INTERVAL_MIN} ms`)
  if (server.error) errors.serverIntervalMs = server.error
  const slots = parseNumber(v.slotsIntervalMs, SLOTS_INTERVAL_MIN, `${SLOTS_INTERVAL_MIN} ms`)
  if (slots.error) errors.slotsIntervalMs = slots.error
  const hardware = parseNumber(v.hardwareIntervalMs, HARDWARE_INTERVAL_MIN, `${HARDWARE_INTERVAL_MIN} ms`)
  if (hardware.error) errors.hardwareIntervalMs = hardware.error
  const storage = parseNumber(v.storageIntervalMs, STORAGE_INTERVAL_MIN, `${STORAGE_INTERVAL_MIN} ms`)
  if (storage.error) errors.storageIntervalMs = storage.error
  const retention = parseNumber(v.retentionHours, RETENTION_MIN_HOURS, `${RETENTION_MIN_HOURS} h`)
  if (retention.error) errors.retentionHours = retention.error

  if (Object.keys(errors).length > 0) return { errors, config: null }

  return {
    errors,
    config: {
      serverUrl: url,
      profiles: v.profiles.map((p) => ({ name: p.name.trim(), serverUrl: p.serverUrl.trim() })),
      activeProfile: v.activeProfile.trim(),
      serverIntervalMs: server.value ?? DEFAULT_CONFIG.serverIntervalMs,
      slotsIntervalMs: slots.value ?? DEFAULT_CONFIG.slotsIntervalMs,
      hardwareIntervalMs: hardware.value ?? DEFAULT_CONFIG.hardwareIntervalMs,
      storageIntervalMs: storage.value ?? DEFAULT_CONFIG.storageIntervalMs,
      retentionHours: retention.value ?? DEFAULT_CONFIG.retentionHours,
    },
  }
}

/** Props for {@link Field}. */
interface FieldProps {
  label: string
  hint?: string
  error?: string
  id?: string
  children: ReactNode
}

/** A labelled form control with inline validation feedback. */
function Field({ label, hint, error, id, children }: FieldProps) {
  return (
    <div>
      <label htmlFor={id}>{label}</label>
      {children}
      {error ? (
        <div className="error-message" style={{ color: 'var(--danger)', fontSize: 12, marginTop: 4 }}>
          {error}
        </div>
      ) : hint ? (
        <div className="muted" style={{ fontSize: 12, marginTop: 4 }}>
          {hint}
        </div>
      ) : null}
    </div>
  )
}

function Settings() {
  const { config, loading, saving, error, save, testUrl } = useSettings()
  const [form, setForm] = useState<FormValues>(() => toForm(DEFAULT_CONFIG))
  const [dirty, setDirty] = useState(false)
  const [errors, setErrors] = useState<FieldErrors>({})
  const [notice, setNotice] = useState<{ tone: Tone; message: string } | null>(null)
  const [test, setTest] = useState<ConnectionTest | null>(null)
  const [testing, setTesting] = useState(false)
  const [pruning, setPruning] = useState(false)
  const [pruned, setPruned] = useState<number | null>(null)

  useEffect(() => {
    if (config && !dirty) setForm(toForm(config))
  }, [config, dirty])

  function update(key: keyof FormValues, value: string) {
    setForm((f) => ({ ...f, [key]: value }))
    setDirty(true)
  }

  /** Edits the server URL of the active profile, keeping `form.serverUrl` in sync. */
  function updateServerUrl(value: string) {
    setForm((f) => {
      const next = [...f.profiles]
      const activeIndex = next.findIndex((p) => p.name === f.activeProfile)
      if (activeIndex >= 0) {
        next[activeIndex] = { ...next[activeIndex], serverUrl: value }
      }
      return { ...f, serverUrl: value, profiles: next }
    })
    setDirty(true)
  }

  function updateProfileName(index: number, value: string) {
    setForm((f) => {
      const next = [...f.profiles]
      next[index] = { ...next[index], name: value }
      return { ...f, profiles: next }
    })
    setDirty(true)
  }

  function updateProfileUrl(index: number, value: string) {
    setForm((f) => {
      const next = [...f.profiles]
      next[index] = { ...next[index], serverUrl: value }
      const updated = { ...f, profiles: next }
      if (next[index].name === f.activeProfile) {
        updated.serverUrl = value
      }
      return updated
    })
    setDirty(true)
  }

  function switchActiveProfile(value: string) {
    setForm((f) => {
      const profile = f.profiles.find((p) => p.name === value)
      return {
        ...f,
        activeProfile: value,
        serverUrl: profile ? profile.serverUrl : f.serverUrl,
      }
    })
    setDirty(true)
  }

  function addProfile() {
    setForm((f) => {
      const existing = new Set(f.profiles.map((p) => p.name))
      let n = 1
      while (existing.has(`Profile ${n}`)) n++
      return {
        ...f,
        profiles: [...f.profiles, { name: `Profile ${n}`, serverUrl: 'http://localhost:8080' }],
      }
    })
    setDirty(true)
  }

  function removeProfile(index: number) {
    setForm((f) => ({
      ...f,
      profiles: f.profiles.filter((_, i) => i !== index),
    }))
    setDirty(true)
  }

  async function onSave() {
    const result = validate(form)
    setErrors(result.errors)
    setNotice(null)
    if (result.config === null) return
    // Frontend contract: serverUrl is the active profile's URL.
    const configToSave = {
      ...result.config,
      serverUrl: form.profiles.find((p) => p.name === form.activeProfile)?.serverUrl ?? result.config.serverUrl,
    }
    try {
      await save(configToSave)
      setDirty(false)
      setNotice({ tone: 'ok', message: 'Settings saved.' })
    } catch (e) {
      setNotice({ tone: 'danger', message: describeError(e) })
    }
  }

  async function onTest() {
    setTesting(true)
    setTest(null)
    setNotice(null)
    try {
      const url = form.serverUrl.trim()
      setTest(await testUrl(url === '' ? undefined : url))
    } catch (e) {
      setNotice({ tone: 'danger', message: describeError(e) })
    } finally {
      setTesting(false)
    }
  }

  async function onPrune() {
    setPruning(true)
    setNotice(null)
    try {
      setPruned(await pruneHistory())
    } catch (e) {
      setNotice({ tone: 'danger', message: describeError(e) })
    } finally {
      setPruning(false)
    }
  }

  if (loading && config === null) return <Loading label="Loading settings…" />

  return (
    <div className="stack">
      {error ? <ErrorBanner message={error} /> : null}

      <Panel title="Settings" subtitle="Persisted and applied live">
        <div className="stack">
          <Field label="Active profile" id="active-profile" error={errors.activeProfile}>
            <select
              id="active-profile"
              value={form.activeProfile}
              onChange={(e) => switchActiveProfile(e.target.value)}
            >
              {form.profiles.map((p) => (
                <option key={p.name} value={p.name}>{p.name}</option>
              ))}
            </select>
          </Field>

          <Field label="Server URL" id="server-url" hint="http or https" error={errors.serverUrl}>
            <input
              id="server-url"
              type="text"
              value={form.serverUrl}
              onChange={(e) => updateServerUrl(e.target.value)}
              spellCheck={false}
            />
          </Field>

          <Field label="Profiles" error={errors.profiles}>
            {form.profiles.map((profile, index) => (
              <div key={index} className="row" style={{ gap: 8, alignItems: 'center', marginBottom: 8 }}>
                <Field label="Name">
                  <input
                    type="text"
                    value={profile.name}
                    onChange={(e) => updateProfileName(index, e.target.value)}
                    spellCheck={false}
                  />
                </Field>
                <Field label="URL">
                  <input
                    type="text"
                    value={profile.serverUrl}
                    onChange={(e) => updateProfileUrl(index, e.target.value)}
                    spellCheck={false}
                  />
                </Field>
                <button
                  type="button"
                  className="btn"
                  disabled={form.profiles.length <= 1 || profile.name === form.activeProfile}
                  onClick={() => removeProfile(index)}
                >
                  Remove
                </button>
              </div>
            ))}
            <button type="button" className="btn" onClick={addProfile}>
              Add profile
            </button>
          </Field>

          <div className="grid cols-2">
            <Field label="Server interval (ms)" hint={`min ${SERVER_INTERVAL_MIN}`} error={errors.serverIntervalMs}>
              <input
                type="number"
                min={SERVER_INTERVAL_MIN}
                step={50}
                value={form.serverIntervalMs}
                onChange={(e) => update('serverIntervalMs', e.target.value)}
              />
            </Field>
            <Field label="Slots interval (ms)" hint={`min ${SLOTS_INTERVAL_MIN}`} error={errors.slotsIntervalMs}>
              <input
                type="number"
                min={SLOTS_INTERVAL_MIN}
                step={50}
                value={form.slotsIntervalMs}
                onChange={(e) => update('slotsIntervalMs', e.target.value)}
              />
            </Field>
            <Field label="Hardware interval (ms)" hint={`min ${HARDWARE_INTERVAL_MIN}`} error={errors.hardwareIntervalMs}>
              <input
                type="number"
                min={HARDWARE_INTERVAL_MIN}
                step={50}
                value={form.hardwareIntervalMs}
                onChange={(e) => update('hardwareIntervalMs', e.target.value)}
              />
            </Field>
            <Field label="Storage interval (ms)" hint={`min ${STORAGE_INTERVAL_MIN}`} error={errors.storageIntervalMs}>
              <input
                type="number"
                min={STORAGE_INTERVAL_MIN}
                step={500}
                value={form.storageIntervalMs}
                onChange={(e) => update('storageIntervalMs', e.target.value)}
              />
            </Field>
            <Field label="Retention (hours)" hint={`min ${RETENTION_MIN_HOURS}`} error={errors.retentionHours}>
              <input
                type="number"
                min={RETENTION_MIN_HOURS}
                step={1}
                value={form.retentionHours}
                onChange={(e) => update('retentionHours', e.target.value)}
              />
            </Field>
          </div>

          <div className="row" style={{ flexWrap: 'wrap' }}>
            <button type="button" className="btn btn-primary" onClick={onSave} disabled={saving}>
              {saving ? 'Saving…' : 'Save'}
            </button>
            <button type="button" className="btn" onClick={onTest} disabled={testing}>
              {testing ? 'Testing…' : 'Test connection'}
            </button>
            <button type="button" className="btn" onClick={onPrune} disabled={pruning}>
              {pruning ? 'Pruning…' : 'Prune history now'}
            </button>
          </div>

          {notice ? (
            notice.tone === 'danger' ? (
              <ErrorBanner message={notice.message} />
            ) : (
              <Badge tone="ok">{notice.message}</Badge>
            )
          ) : null}

          {pruned !== null ? (
            <div className="muted">
              Pruned {pruned} row{pruned === 1 ? '' : 's'}.
            </div>
          ) : null}
        </div>
      </Panel>

      {test ? (
        <Panel title="Connection test">
          <MetricRow
            label="Result"
            value={<Badge tone={test.ok ? 'ok' : 'danger'}>{test.ok ? 'ok' : 'failed'}</Badge>}
          />
          <MetricRow label="Message" value={test.message} />
          <MetricRow label="Role" value={test.role ?? PLACEHOLDER} />
          <MetricRow label="Build" value={test.buildInfo ?? PLACEHOLDER} />
          <MetricRow label="Models" value={test.modelCount ?? PLACEHOLDER} />
        </Panel>
      ) : null}
    </div>
  )
}

export default Settings
