# Times codestats on Windows against its last release and mezura, and prints the numbers behind the README's Windows tables.
# It only writes inside %USERPROFILE%\cs-bench and changes no settings.
# Run with: powershell -ExecutionPolicy Bypass -File bench-windows.ps1 [-New <cs.exe>]
# -New times that binary instead of the latest nightly, such as a build of a commit the nightly doesn't have yet.

param([string]$New)

$root = Join-Path $env:USERPROFILE 'cs-bench'
$results = Join-Path $root 'win-bench-results.txt'
$null = New-Item -ItemType Directory -Force -Path $root
Set-Location $root
Set-Content -Path $results -Value "codestats Windows benchmark, $(Get-Date -Format s)"

function Say($text) {
	Write-Output $text
	Add-Content -Path $results -Value $text
}

# Machine state, which decides how far these numbers can be trusted.
$os = Get-CimInstance Win32_OperatingSystem
$cpu = Get-CimInstance Win32_Processor | Select-Object -First 1
Say "OS: $($os.Caption) $($os.Version)"
Say "CPU: $($cpu.Name), $($cpu.NumberOfCores) cores, $($cpu.NumberOfLogicalProcessors) threads"
Say "Memory: $([math]::Round($os.TotalVisibleMemorySize / 1MB)) GB"
Say "Architecture: $env:PROCESSOR_ARCHITECTURE"
Say "Power plan: $(powercfg /getactivescheme)"
try {
	$status = Get-MpComputerStatus -ErrorAction Stop
	Say "Defender real-time protection: $($status.RealTimeProtectionEnabled)"
} catch {
	Say "Defender real-time protection: could not be read"
}
try {
	$preference = Get-MpPreference -ErrorAction Stop
	Say "Defender process exclusions: $($preference.ExclusionProcess -join ', ')"
	Say "Defender path exclusions: $($preference.ExclusionPath -join ', ')"
} catch {
	Say "Defender exclusions: could not be read"
}

function Fetch($url, $file) {
	if (Test-Path $file) { return }
	Write-Output "Downloading $file"
	curl.exe -L -f -s -S -o "$file.part" $url
	if ($LASTEXITCODE -ne 0) { throw "Couldn't download $url" }
	Move-Item "$file.part" $file
}

# The binaries. mezura only ships for x86-64, which runs under emulation on ARM.
$target = if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') { 'cs-aarch64-pc-windows-gnullvm.exe' } else { 'cs-x86_64-pc-windows-gnu.exe' }
$base = 'https://github.com/trypsynth/codestats/releases/download'
Remove-Item 'cs-new.exe' -ErrorAction SilentlyContinue
if ($New) {
	Copy-Item -Path $New -Destination 'cs-new.exe'
	Say "New binary: $New"
} else {
	# Always the latest nightly, unlike the pinned downloads below. The nightly is rebuilt on every push, so its commit is recorded.
	Fetch "$base/nightly/$target" 'cs-new.exe'
	try {
		$nightly = Invoke-RestMethod -Uri 'https://api.github.com/repos/trypsynth/codestats/releases/tags/nightly' -Headers @{ 'User-Agent' = 'codestats-bench' }
		$commit = if ($nightly.body -match 'Commit:\*\*\s*([0-9a-f]{7,40})') { $Matches[1].Substring(0, 7) } else { 'unknown' }
		Say "Nightly commit: $commit"
	} catch {
		Say 'Nightly commit: could not be read'
	}
}
Fetch "$base/v1.1.0/$target" 'cs-110.exe'
Fetch 'https://github.com/subamanis/mezura/releases/download/v3.5.0/mezura-v3.5.0-windows-x86_64.zip' 'mezura-3.5.0.zip'
if (-not (Test-Path 'mezura-3.5.0')) {
	Expand-Archive -Path 'mezura-3.5.0.zip' -DestinationPath 'mezura-3.5.0'
}
$mezura = (Get-ChildItem -Path 'mezura-3.5.0' -Recurse -Filter 'mezura.exe' | Select-Object -First 1).FullName
$new = Join-Path $root 'cs-new.exe'
$old = Join-Path $root 'cs-110.exe'

# The repositories, at the commits scripts/bench.sh pins.
$repositories = @(
	@{ Name = 'ripgrep'; Repo = 'BurntSushi/ripgrep'; Sha = '91f5d22ef48bde459d90a8c7dbeab416e97d0f2b'; Rounds = 40 },
	@{ Name = 'cli'; Repo = 'cli/cli'; Sha = '9490d204a2ba95434153dedce99898ce9f281426'; Rounds = 30 },
	@{ Name = 'cpython'; Repo = 'python/cpython'; Sha = 'ef1fce0a69a19360643fb82ea3545ede21495c73'; Ext = 'py,c,h,html,js,bat,sh,m'; Rounds = 20 },
	@{ Name = 'kubernetes'; Repo = 'kubernetes/kubernetes'; Sha = 'a972b672c7612c4892557f3d6d2a952c741c5d1f'; Ext = 'go,sh,py'; Rounds = 10 }
)
foreach ($repository in $repositories) {
	$dir = Join-Path $root $repository.Name
	if (Test-Path (Join-Path $dir '.unpacked')) { continue }
	Fetch "https://codeload.github.com/$($repository.Repo)/tar.gz/$($repository.Sha)" "$($repository.Name).tar.gz"
	Write-Output "Unpacking $($repository.Name)"
	$null = New-Item -ItemType Directory -Force -Path $dir
	# A few names can't exist on NTFS. tar skips them, and every tool then sees the same files.
	tar.exe -xzf "$($repository.Name).tar.gz" --strip-components=1 -C $dir 2>$null
	Set-Content -Path (Join-Path $dir '.unpacked') -Value 'done'
}
$empty = Join-Path $root 'empty'
$null = New-Item -ItemType Directory -Force -Path $empty

function Invoke-Tool($exe, $argumentLine) {
	$info = New-Object System.Diagnostics.ProcessStartInfo
	$info.FileName = $exe
	$info.Arguments = $argumentLine
	$info.UseShellExecute = $false
	$info.CreateNoWindow = $true
	$info.RedirectStandardOutput = $true
	$info.RedirectStandardError = $true
	$info.WorkingDirectory = $root
	$watch = [System.Diagnostics.Stopwatch]::StartNew()
	$process = [System.Diagnostics.Process]::Start($info)
	$outputTask = $process.StandardOutput.ReadToEndAsync()
	$errorTask = $process.StandardError.ReadToEndAsync()
	$process.WaitForExit()
	$watch.Stop()
	return @{ Ms = $watch.Elapsed.TotalMilliseconds; Output = $outputTask.Result; Error = $errorTask.Result; Code = $process.ExitCode }
}

function Get-Median($values) {
	$sorted = @($values | Sort-Object)
	$middle = [math]::Floor($sorted.Count / 2)
	if ($sorted.Count % 2 -eq 1) { return $sorted[$middle] }
	return ($sorted[$middle - 1] + $sorted[$middle]) / 2
}

# Runs each command in a new random order every round, so that heat and caches fall on all of them alike.
function Compare-Tools($title, $commands, $rounds) {
	Say ''
	Say $title
	Write-Output "  measuring, $rounds rounds"
	foreach ($command in $commands) { $null = Invoke-Tool $command.Exe $command.Line }
	foreach ($command in $commands) { $null = Invoke-Tool $command.Exe $command.Line }
	$times = @{}
	foreach ($command in $commands) { $times[$command.Name] = New-Object System.Collections.ArrayList }
	for ($round = 0; $round -lt $rounds; $round++) {
		foreach ($command in ($commands | Sort-Object { Get-Random })) {
			$run = Invoke-Tool $command.Exe $command.Line
			if ($run.Code -ne 0) { Say "  $($command.Name) exited with $($run.Code): $($run.Error)" }
			$null = $times[$command.Name].Add($run.Ms)
		}
	}
	foreach ($command in $commands) {
		$median = Get-Median $times[$command.Name]
		$fastest = ($times[$command.Name] | Measure-Object -Minimum).Minimum
		Say ("  {0}: median {1:N2} ms, fastest {2:N2} ms" -f $command.Name, $median, $fastest)
	}
}

function Get-Counts($exe, $line, $isMezura) {
	$run = Invoke-Tool $exe $line
	try {
		$json = $run.Output | ConvertFrom-Json
		if ($isMezura) { return "$($json.total.files) files, $($json.total.lines) lines" }
		return "$($json.totals.file_count) files, $($json.totals.lines.total) lines"
	} catch {
		return "counts could not be read (exit code $($run.Code))"
	}
}

Say "codestats new: $((Invoke-Tool $new '--version').Output.Trim())"
Say "codestats 1.1.0: $((Invoke-Tool $old '--version').Output.Trim())"
Say "mezura: $((Invoke-Tool $mezura '--version').Output.Trim())"

$newJson = '-o json-compact'
$mezuraJson = '--output json'
$mezuraSame = '--no-gitignore --no-ignore-files --search-in-dotted --no-local --no-default-config --hide keywords,tests --count-minified --count-generated --count-not-code --counting region --no-shebang'

Compare-Tools 'Startup: --version' @(
	@{ Name = 'codestats new'; Exe = $new; Line = '--version' },
	@{ Name = 'codestats 1.1.0'; Exe = $old; Line = '--version' },
	@{ Name = 'mezura'; Exe = $mezura; Line = '--version' }
) 50
Compare-Tools 'Startup: an empty directory' @(
	@{ Name = 'codestats new'; Exe = $new; Line = "`"$empty`" $newJson" },
	@{ Name = 'codestats 1.1.0'; Exe = $old; Line = "`"$empty`" $newJson" },
	@{ Name = 'mezura'; Exe = $mezura; Line = "`"$empty`" $mezuraJson" }
) 50

foreach ($repository in $repositories) {
	$dir = Join-Path $root $repository.Name
	Compare-Tools "$($repository.Name), default settings" @(
		@{ Name = 'codestats new'; Exe = $new; Line = "`"$dir`" $newJson" },
		@{ Name = 'codestats 1.1.0'; Exe = $old; Line = "`"$dir`" $newJson" },
		@{ Name = 'mezura'; Exe = $mezura; Line = "`"$dir`" $mezuraJson" }
	) $repository.Rounds
	$newDefault = "`"$dir`" $newJson"
	$mezuraDefault = "`"$dir`" $mezuraJson"
	Say "  counted by codestats new: $(Get-Counts $new $newDefault $false)"
	Say "  counted by codestats 1.1.0: $(Get-Counts $old $newDefault $false)"
	Say "  counted by mezura: $(Get-Counts $mezura $mezuraDefault $true)"

	# mezura's --languages leaves out Markdown and TOML, so only repositories without them in Ext get equal-work runs.
	if (-not $repository.Ext) { continue }
	$newSame = "`"$dir`" --ext $($repository.Ext) --no-gitignore --hidden --include-generated --no-gitattributes --no-config $newJson"
	$mezuraEqual = "`"$dir`" --languages $($repository.Ext) $mezuraSame $mezuraJson"
	Compare-Tools "$($repository.Name), equal work" @(
		@{ Name = 'codestats new'; Exe = $new; Line = $newSame },
		@{ Name = 'mezura'; Exe = $mezura; Line = $mezuraEqual }
	) $repository.Rounds
	Say "  counted by codestats new: $(Get-Counts $new $newSame $false)"
	Say "  counted by mezura: $(Get-Counts $mezura $mezuraEqual $true)"
}

Say ''
Say 'Done.'
Write-Output "Results are in $results"
