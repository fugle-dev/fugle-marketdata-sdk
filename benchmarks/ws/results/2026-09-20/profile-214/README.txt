#214: where the new SDK's user CPU goes (2026-09-20, same machine as cpu-rerun/, main f7c31e7 tree)

threads-js-{old,new}-200k.txt   js/bench-{old,new}.js at 200,000 frames, 3 runs, with `ps -M -p <pid>`
                                (per-thread STIME/UTIME, 10 ms resolution) printed before the JSON line.
                                Old SDK: everything on the main thread (0.59 s user / 0.20 s sys).
                                New SDK (3 runs): main 0.44-0.45 / 0.15, tokio worker 0.46 / 0.32-0.33,
                                stream reader 0.15-0.17 / 0.25-0.29, +0.05 elsewhere. The COMMAND
                                column is trimmed to the script name.
sample-js-new-1500k.txt         macOS `sample` (3 s @ 1 ms) of the new JS client mid-burst; summary in
sample-summary.txt              sample-summary.txt.
prototype-data-rawvalue.patch   the measurement-only core/uniffi change described in
prototype-results.txt           prototype-results.txt, with the numbers it produced.

The bench clients used for the ps -M runs are copies of js/bench-{old,new}.js with absolute `require`
paths and this inserted right after `const endCpu = process.cpuUsage(startCpu);` in finish():

    try {
      const ps = require('child_process').execSync(`ps -M -p ${process.pid}`).toString();
      process.stderr.write(ps);
    } catch (e) {}

Nothing else differs. Run per iteration: start ws-mock-server.js (--count 200000 --rate 0 --warmup 1000),
then `node --expose-gc <client> --url ws://localhost:<port> --timeout 60000`.
