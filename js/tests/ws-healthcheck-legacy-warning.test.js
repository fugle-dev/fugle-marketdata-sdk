/**
 * `healthCheck` fields of `@fugle/marketdata` 1.x that 3.0 does not have
 * (`pingInterval`, `maxMissedPongs`) are still ignored, but no longer
 * silently: the first client given one emits a process warning (#262).
 */

const { spawn } = require('child_process');
const path = require('path');

const PKG = path.resolve(__dirname, '..');

/**
 * Build one client per entry of `healthChecks` in a child Node process (the
 * warning is once per process, so each case needs its own) and resolve with
 * the process warnings it saw and the error code each constructor threw.
 */
function construct(healthChecks) {
  const script = `
    const { WebSocketClient } = require('./');
    const warnings = [];
    process.on('warning', (w) => warnings.push({ name: w.name, code: w.code, message: w.message }));
    const thrown = [${healthChecks.join(', ')}].map((healthCheck) => {
      try {
        const ws = new WebSocketClient({ apiKey: 'test-key', healthCheck });
        return ws.stock ? null : 'no stock client';
      } catch (e) {
        return e.code;
      }
    });
    // process.emitWarning() delivers on a later tick.
    setImmediate(() => console.log('RESULT ' + JSON.stringify({ warnings, thrown })));
  `;
  return new Promise((resolve) => {
    const child = spawn(process.execPath, ['-e', script], { cwd: PKG, stdio: ['ignore', 'pipe', 'pipe'] });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (chunk) => {
      stdout += chunk;
    });
    child.stderr.on('data', (chunk) => {
      stderr += chunk;
    });
    const timer = setTimeout(() => child.kill('SIGKILL'), 10000);
    child.on('exit', (code) => {
      clearTimeout(timer);
      const line = stdout.split('\n').find((l) => l.startsWith('RESULT '));
      resolve({ code, stderr, ...(line ? JSON.parse(line.slice('RESULT '.length)) : {}) });
    });
  });
}

const NEW_FIELDS = /heartbeatTimeoutMs.*probeEnabled.*idleProbeAfterMs.*probeTimeoutMs/;

describe('healthCheck 1.x fields (#262)', () => {
  test('both legacy fields: one warning naming them and the 3.0 fields', async () => {
    const run = await construct(['{ enabled: true, pingInterval: 30000, maxMissedPongs: 2 }']);

    expect(run).toMatchObject({ code: 0, thrown: [null] });
    expect(run.warnings).toHaveLength(1);
    expect(run.warnings[0]).toMatchObject({
      name: 'FugleHealthCheckWarning',
      code: 'FUGLE_HEALTH_CHECK_LEGACY_OPTIONS',
    });
    expect(run.warnings[0].message).toMatch(
      /^healthCheck\.pingInterval and healthCheck\.maxMissedPongs do not exist in @fugle\/marketdata 3\.0 and were ignored\./,
    );
    expect(run.warnings[0].message).toMatch(NEW_FIELDS);
    // Node prints it on stderr unless the application handles warnings.
    expect(run.stderr).toMatch(/FugleHealthCheckWarning/);
  });

  test.each([
    ['pingInterval', 'maxMissedPongs'],
    ['maxMissedPongs', 'pingInterval'],
  ])('%s alone: the warning names only it', async (given, other) => {
    const run = await construct([`{ ${given}: 5 }`]);

    expect(run).toMatchObject({ code: 0, thrown: [null] });
    expect(run.warnings).toHaveLength(1);
    expect(run.warnings[0].message).toMatch(
      new RegExp(`^healthCheck\\.${given} does not exist in @fugle/marketdata 3\\.0 and was ignored\\.`),
    );
    expect(run.warnings[0].message).not.toContain(other);
    expect(run.warnings[0].message).toMatch(NEW_FIELDS);
  });

  test('warns once per process, for the first client given a legacy field', async () => {
    const run = await construct(['{ pingInterval: 30000 }', '{ maxMissedPongs: 2 }', '{ pingInterval: 30000 }']);

    expect(run).toMatchObject({ code: 0, thrown: [null, null, null] });
    expect(run.warnings).toHaveLength(1);
    expect(run.warnings[0].message).toMatch(/^healthCheck\.pingInterval does not exist/);
  });

  test('no warning without a legacy field', async () => {
    const run = await construct([
      'undefined',
      '{}',
      '{ enabled: false }',
      '{ probeEnabled: true, idleProbeAfterMs: 5000, probeTimeoutMs: 1000 }',
      // Not passed, as for the fields 3.0 has.
      '{ pingInterval: undefined, maxMissedPongs: null }',
    ]);

    expect(run).toMatchObject({ code: 0, thrown: [null, null, null, null, null], warnings: [] });
    expect(run.stderr).toBe('');
  });

  test('the legacy fields are not validated, the 3.0 ones still are', async () => {
    const run = await construct([
      // Neither a number: ignored all the same.
      "{ pingInterval: 'soon', maxMissedPongs: {} }",
      '{ pingInterval: 30000, heartbeatTimeoutMs: 4999 }',
    ]);

    expect(run).toMatchObject({ code: 0, thrown: [null, 1004] });
    expect(run.warnings).toHaveLength(1);
  });

  test('a constructor that throws does not use up the warning', async () => {
    const run = await construct([
      '{ pingInterval: 30000, heartbeatTimeoutMs: 4999 }',
      '{ maxMissedPongs: 2 }',
    ]);

    expect(run).toMatchObject({ code: 0, thrown: [1004, null] });
    expect(run.warnings).toHaveLength(1);
    expect(run.warnings[0].message).toMatch(/^healthCheck\.maxMissedPongs does not exist/);
  });

  test('a legacy field whose getter throws is neither an error nor a warning', async () => {
    const run = await construct(["{ get pingInterval() { throw new Error('getter'); } }"]);

    expect(run).toMatchObject({ code: 0, thrown: [null], warnings: [] });
  });
});
