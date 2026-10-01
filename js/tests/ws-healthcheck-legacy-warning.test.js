/**
 * `healthCheck` fields of `@fugle/marketdata` 1.x that 3.0 does not have
 * (`pingInterval`, `maxMissedPongs`) are still ignored, but no longer
 * silently: the first client given one emits a process warning (#262).
 */

const { spawn } = require('child_process');
const path = require('path');

const PKG = path.resolve(__dirname, '..');

/**
 * Run `script` in a child Node process (the warning is once per process, so
 * each case needs its own) and resolve with its exit code, its stderr and the
 * object printed on its `RESULT ` line. Rejects if it has not exited in 20 s.
 */
function runChild(script) {
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, ['-e', script], { cwd: PKG, stdio: ['ignore', 'pipe', 'pipe'] });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (chunk) => {
      stdout += chunk;
    });
    child.stderr.on('data', (chunk) => {
      stderr += chunk;
    });
    let timedOut = false;
    const timer = setTimeout(() => {
      timedOut = true;
      child.kill('SIGKILL');
    }, 20000);
    child.on('exit', (code) => {
      clearTimeout(timer);
      if (timedOut) {
        reject(new Error(`child timed out after 20 s; stderr:\n${stderr}`));
        return;
      }
      const line = stdout.split('\n').find((l) => l.startsWith('RESULT '));
      resolve({ code, stderr, ...(line ? JSON.parse(line.slice('RESULT '.length)) : {}) });
    });
  });
}

/**
 * Build one client per entry of `healthChecks` in a child and resolve with
 * the process warnings it saw and the error code each constructor threw.
 */
function construct(healthChecks) {
  return runChild(`
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
  `);
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
    expect(run.warnings[0].message).toContain('default 35000 ms');
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
      // Neither undefined nor null counts as passed.
      '{ pingInterval: undefined, maxMissedPongs: null }',
    ]);

    expect(run).toMatchObject({ code: 0, thrown: [null, null, null, null, null], warnings: [] });
    expect(run.stderr).not.toMatch(/FugleHealthCheckWarning/);
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

  test('a getter that throws on one legacy field: the warning names only the other', async () => {
    const run = await construct(["{ get pingInterval() { throw new Error('getter'); }, maxMissedPongs: 2 }"]);

    expect(run).toMatchObject({ code: 0, thrown: [null] });
    expect(run.warnings).toHaveLength(1);
    expect(run.warnings[0].message).toMatch(/^healthCheck\.maxMissedPongs does not exist/);
    expect(run.warnings[0].message).not.toContain('pingInterval');
  });

  // The flag is the process's, but the warning goes to the `process` of the
  // thread whose constructor ran: a worker's warning is not seen on the main
  // thread, which then no longer warns either.
  test('a worker thread that warns first leaves the main thread without a warning', async () => {
    const run = await runChild(`
      const { Worker } = require('worker_threads');
      const { WebSocketClient } = require('./');
      const warnings = [];
      process.on('warning', (w) => warnings.push({ name: w.name, message: w.message }));
      const worker = new Worker(
        "const warnings = []; process.on('warning', (w) => warnings.push({ name: w.name, message: w.message }));" +
          "const { WebSocketClient } = require('./');" +
          "new WebSocketClient({ apiKey: 'test-key', healthCheck: { pingInterval: 30000 } });" +
          "setImmediate(() => require('worker_threads').parentPort.postMessage(warnings));",
        { eval: true },
      );
      worker.on('message', (inWorker) => {
        new WebSocketClient({ apiKey: 'test-key', healthCheck: { maxMissedPongs: 2 } });
        setImmediate(() => console.log('RESULT ' + JSON.stringify({ inWorker, inMain: warnings })));
      });
    `);

    expect(run).toMatchObject({ code: 0, inMain: [] });
    expect(run.inWorker).toHaveLength(1);
    expect(run.inWorker[0].name).toBe('FugleHealthCheckWarning');
    expect(run.inWorker[0].message).toMatch(/^healthCheck\.pingInterval does not exist/);
  });
});

// What `healthCheck` itself accepts: an object of its own fields. Anything
// else is a `TypeError` naming the field (#294); before, napi read a
// non-object as `{}` and threw its own conversion errors for bad fields.
describe('healthCheck of the wrong type (#294)', () => {
  const { WebSocketClient } = require('../');
  const prefix = 'WebSocketClient options: ';
  const ms = 'must be a number of milliseconds';

  test.each([
    [5, 'healthCheck must be an object like { enabled: true }, got number 5'],
    ['x', 'healthCheck must be an object like { enabled: true }, got string'],
    [[], 'healthCheck must be an object like { enabled: true }, got array'],
    [true, 'healthCheck must be an object like { enabled: true }, got boolean'],
    [new Date(0), 'healthCheck must be an object like { enabled: true }, got object (Date)'],
    [{ enabled: 'x' }, 'healthCheck.enabled must be a boolean, got string'],
    [{ probeEnabled: 1 }, 'healthCheck.probeEnabled must be a boolean, got number 1'],
    [{ heartbeatTimeoutMs: 'x' }, `healthCheck.heartbeatTimeoutMs ${ms}, got string`],
    [{ idleProbeAfterMs: {} }, `healthCheck.idleProbeAfterMs ${ms}, got object`],
    [{ probeTimeoutMs: [] }, `healthCheck.probeTimeoutMs ${ms}, got array`],
  ])('%j throws a TypeError', (healthCheck, message) => {
    let error;
    try {
      new WebSocketClient({ apiKey: 'test-key', healthCheck });
    } catch (e) {
      error = e;
    }
    // `name`, not `instanceof`: jest's sandbox has its own `TypeError`.
    expect({ name: error.name, message: error.message }).toEqual({ name: 'TypeError', message: prefix + message });
  });
});
