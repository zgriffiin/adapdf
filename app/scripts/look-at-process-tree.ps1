# One look at a process tree, for `axe-webview.mjs --watch` (rows I-07 and N-03).
#
# Writes JSON: the processes under the process with the id RootPid (itself and all its
# descendants), and the TCP and UDP endpoints that those processes own (`netstat -ano`).
# It only reads. It runs in Windows PowerShell 5.1, which every Windows 11 has.
param([Parameter(Mandatory)][int]$RootPid)

$all = @(Get-CimInstance Win32_Process)
$set = @{ $RootPid = 1 }
do {
    $added = 0
    foreach ($p in $all) {
        if ($set.ContainsKey([int]$p.ParentProcessId) -and -not $set.ContainsKey([int]$p.ProcessId)) {
            $set[[int]$p.ProcessId] = 1
            $added++
        }
    }
} while ($added -gt 0)

$procs = @($all | Where-Object { $set.ContainsKey([int]$_.ProcessId) } | ForEach-Object {
    [pscustomobject]@{ pid = [int]$_.ProcessId; ppid = [int]$_.ParentProcessId; name = $_.Name; exe = $_.ExecutablePath; cmd = $_.CommandLine }
})

$conns = @(netstat -ano | ForEach-Object {
    $f = ($_.Trim() -split '\s+')
    if ($f.Count -ge 4 -and ($f[0] -eq 'TCP' -or $f[0] -eq 'UDP') -and $set.ContainsKey([int]$f[-1])) {
        $state = ''
        if ($f[0] -eq 'TCP') { $state = $f[3] }
        [pscustomobject]@{ pid = [int]$f[-1]; proto = $f[0]; local = $f[1]; remote = $f[2]; state = $state }
    }
})

[pscustomobject]@{ procs = $procs; conns = $conns } | ConvertTo-Json -Depth 4 -Compress
