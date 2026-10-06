---
name: cpu-profile
description: >-
  Sample a Windows release binary with Visual Studio's CPU profiler and
  resolve stacks, including ntdll frames. Use when the user asks for a
  profile, a flame graph, a hot path, or why convx is slow compared with
  Qhull.
---

# CPU profile on this machine

The agent shell is medium integrity. Administrators is deny-only. A kernel logger does not start.

Do not run `wpr` or `xperf -on`. Do not switch to in-process timers because a session failed to start.

## Collect

Build the release binary with debug info so the PDB matches the image.

```powershell
$env:CARGO_PROFILE_RELEASE_DEBUG = "1"
cargo build --release
```

`VSDiagnostics.exe` is under Visual Studio's Diagnostics Hub collector. `CpuUsageHigh.json` is next to it in `AgentConfigs`. Session id is an integer from 1 to 255.

```powershell
$vs = "C:\Program Files\Microsoft Visual Studio\18\Community\Team Tools\DiagnosticsHub\Collector\VSDiagnostics.exe"
$cfg = "C:\Program Files\Microsoft Visual Studio\18\Community\Team Tools\DiagnosticsHub\Collector\AgentConfigs\CpuUsageHigh.json"
& $vs start 4 "/launch:$exe" "/launchArgs:$args" "/loadConfig:$cfg" "/scratchLocation:$scratch"
```

If the start says the class is not registered, start again with a different integer session id. Do not treat that as the profiler being unavailable.

When the process has exited:

```powershell
& $vs stop 4 "/output:$scratch\cpu.diagsession"
& $vs expandDiagSession "$scratch\cpu.diagsession"
```

The sample ETL is `sc.user_aux.etl` under the expanded directory. The agent samples at 4000 Hz.

## Decode

Point the symbol path at the directory that contains the PDB.

```powershell
$env:_NT_SYMBOL_PATH = $pdbDir
$env:_NT_SYMCACHE_PATH = "$scratch\symcache"
xperf -i $etl -symbols -o "$scratch\profile.txt" -a profile -detail
xperf -i $etl -symbols -o "$scratch\stacks.txt" -a stack -butterfly 50 -process $procName -pid $pid
```

`-o` comes before `-a`. `profile -detail` is the leaf hit. `stack -butterfly` is the inclusive tree. Read a function's own block, the one that contains `***itself***`. Rows above that block are the parent's other callees.

## ntdll

Frames shown as `ntdll.dll!***unknown***` still have an RVA. Resolve it to the nearest export at or below that RVA:

```powershell
dumpbin /exports C:\Windows\System32\ntdll.dll
```

`RtlAllocateHeap` and `RtlFreeHeap` are the usual hot exports. `RtlQueryPerformanceCounter` is `Instant::now`, which means a timer was compiled into the binary. Report that share separately from the hull.

## What to report

Inclusive shares of the process samples, for the functions that own the time, and the exclusive heap exports. A leaf list alone hides a caller that spends its time in callees. State the sample count, the rate, and the binary's commit.

## VTune

`vtune.exe` is under `C:\Program Files (x86)\Intel\oneAPI\vtune\latest\bin64`. Use it when the question is which stall, not only which function (#209).

Profile the binary where cargo built it. A copy without its PDB shows `func@0x...` instead of names.

Pin the target, not the collector:

```powershell
& $vtune -collect hotspots -knob sampling-mode=sw -r $result -- cmd /c "start /affinity 10 /b /wait $exe $args"
```

User-mode sampling (`sampling-mode=sw`) runs from the agent shell. It gives shares by function and module at a 10 ms interval, so repeat the build until the run lasts about 10 s.

Hardware event-based sampling (`-collect uarch-exploration`, `-collect memory-access`) needs an elevated process. Write the collection as a script and start it with `Start-Process pwsh -Verb RunAs -Wait`; the owner approves the UAC prompt. Reports run unelevated.

```powershell
& $vtune -report summary -r $result
& $vtune -report hotspots -r $result -group-by function -format csv -csv-delimiter "|"
& $vtune -report gprof-cc -r $result -format text -report-output $file
& $vtune -report callstacks -r $result -filter "module=ntdll.dll" -format csv -csv-delimiter "|"
```

`summary` of a `uarch-exploration` result has the top-down split. `hotspots` by function has the same columns per function. `gprof-cc` has inclusive time. `callstacks` filtered to `ntdll.dll` or `VCRUNTIME140.dll` attributes heap and copy time to the first `convx::` frame.

Inlining folds callees into their caller: the walk and the commit both appeared as `SimplicialHull::absorb`. Read line-level rows with that in mind.

To count allocations, give the scratch binary a `#[global_allocator]` that wraps `System` and counts `alloc` and `realloc`. The count per build and per facet says whether the heap share is many small allocations.

Delete the result directories with the scratch binary.
