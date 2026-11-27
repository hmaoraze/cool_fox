#Requires -Version 7.0
<#
    Attach the staged package to the release named by the tag.

    Called by .github/workflows/release.yml, and runnable here by hand:

        ./scripts/publish-release.ps1 -Tag v0.1.0 -WhatIf      # what it would do
        ./scripts/publish-release.ps1 -Tag v0.1.0              # needs GH_TOKEN

    Every rule below exists because breaking it once cost a CI run: the job
    built for six minutes, failed in five seconds at this step, and the log was
    behind a login.

    * A native command writing to stderr is not an error. Under
      $ErrorActionPreference = 'Stop' the first stderr line terminates the step;
      `gh release view` writes "release not found" there on purpose. So every
      call goes through Invoke-Native, which runs with 'Continue'.
    * The exit code is read immediately after the call, never after a pipeline
      that could have reset it, and $null is failure - not 0.
    * Multi-line output is an array in PowerShell. git prints a tag message one
      line at a time; passed straight to --notes, the first line became the body
      and the rest became file patterns ("no matches found for ``").
    * A function returns what it emits, so this file's helper emits one object
      and everything else writes with Write-Host.
    * `-q` needs `--json` on `gh release view`, otherwise the probe fails and
      every run looks like "no release exists".
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [string] $Tag,
    [string] $Dist = 'dist',
    [switch] $WhatIf
)

$ErrorActionPreference = 'Continue'

function Invoke-Native {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][string] $Command,
        [string[]] $Arguments = @()
    )
    $lines = @()
    $code = 1
    $was = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $lines = @(& $Command @Arguments 2>&1 | ForEach-Object { [string] $_ })
        if ($null -ne $LASTEXITCODE) { $code = [int] $LASTEXITCODE }
    } catch {
        # A command that is not installed lands here; that is a failure, not a
        # crash, and the message is the diagnosis.
        $lines = @($_.Exception.Message)
        $code = 1
    } finally {
        $ErrorActionPreference = $was
    }
    return [pscustomobject]@{
        Code = $code
        Text = (($lines -join "`n").Trim())
    }
}

$package = Get-ChildItem -Path (Join-Path $Dist '*.zip') -ErrorAction SilentlyContinue |
    Select-Object -First 1
if (-not $package) { throw "no package in $Dist" }
Write-Host ("package: {0} ({1:N1} MB)" -f $package.Name, ($package.Length / 1MB))

# One string, not one per line.
$tagText = Invoke-Native git @('tag', '-l', '--format=%(contents:subject)%0a%0a%(contents:body)', $Tag)
$notes = $tagText.Text
Write-Host ('tag message: ' + $(if ($notes) { 'yes' } else { 'none' }))

$probe = Invoke-Native gh @('release', 'view', $Tag, '--json', 'tagName', '-q', '.tagName')
$exists = ($probe.Code -eq 0)
Write-Host ("release exists: {0} {1}" -f $exists, $probe.Text)

if ($WhatIf) {
    $what = if ($exists) { 'upload over' } else { 'create' }
    Write-Host "would $what the release $Tag with $($package.Name)"
    exit 0
}

if ($exists) {
    # A re-run of the same tag replaces the files instead of failing.
    $result = Invoke-Native gh @('release', 'upload', $Tag, $package.FullName, '--clobber')
    if ($result.Code -eq 0 -and $notes) {
        $null = Invoke-Native gh @('release', 'edit', $Tag, '--notes', $notes)
    }
} elseif ($notes) {
    $result = Invoke-Native gh @('release', 'create', $Tag, $package.FullName, '--title', $Tag, '--notes', $notes)
} else {
    $result = Invoke-Native gh @('release', 'create', $Tag, $package.FullName, '--title', $Tag, '--generate-notes')
}

Write-Host '--- gh said ---'
Write-Host $result.Text

if ($result.Code -ne 0) {
    # The step log needs an account to read; the run summary does not.
    if ($env:GITHUB_STEP_SUMMARY) {
        # One string with explicit newlines: a string array outsourced to
        # Out-File is what turned a blank line and a fence into one line.
        $block = @(
            '## the release step failed',
            '',
            '```text',
            $result.Text,
            '```',
            '',
            'A 403 (Resource not accessible by integration) means the workflow token',
            'is read-only: Settings > Actions > General > Workflow permissions >',
            '"Read and write permissions". A permissions block can narrow that',
            'setting, not widen it.'
        ) -join "`n"
        [System.IO.File]::AppendAllText(
            $env:GITHUB_STEP_SUMMARY,
            $block + "`n",
            (New-Object System.Text.UTF8Encoding($false))
        )
    }
    Write-Host 'the release was not published; see the message above'
}

exit $result.Code
