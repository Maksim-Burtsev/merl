# A small ledger: entries, totals and a report.
<#
  .SYNOPSIS
  Prints a ledger report.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [int]$MaxEntries = 1024,
    [switch]$Verbose
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$ratio = 1.5e3
$mask = 0xFF
$size = 10MB
$flags = @($true, $false, $null)
$name = "ledger `"main`" for $env:USERNAME on $(Get-Date -Format 'yyyy-MM-dd')"
$literal = 'it''s $not expanded'
$here = @"
Total: $total
Owner: ${env:USERNAME}
"@
$table = @{ Food = 12; Rent = 900; 'Misc' = 3 }

class Entry {
    [string]$Name
    [int]$Amount = 0
    hidden [string[]]$Tags

    Entry([string]$name, [int]$amount) {
        $this.Name = $name
        $this.Amount = $amount
    }

    [bool] IsCredit() {
        return $this.Amount -gt 0 -and $this.Name.Length -ne 0
    }

    static [Entry] Parse([string]$line) {
        $parts = $line -split ','
        return [Entry]::new($parts[0], [int]$parts[1])
    }
}

enum Kind {
    Credit = 1
    Debit
}

function Get-Total {
    param([Entry[]]$Entries)
    $sum = 0
    foreach ($e in $Entries) {
        if ($null -eq $e) { continue }
        $sum += $e.Amount
    }
    return $sum
}

function Write-Report([Entry[]]$Entries) {
    $n = $Entries.Count
    if ($n -gt $MaxEntries) {
        throw "too many entries: $n"
    } elseif ($n -eq 0) {
        Write-Warning 'empty'
    } else {
        $Entries | Where-Object { $_.IsCredit() } | ForEach-Object {
            '{0,-20} {1,8:N2}' -f $_.Name, $_.Amount
        } | Out-Host
    }
}

$entries = Get-Content -Path $Path | ForEach-Object { [Entry]::Parse($_) }
try {
    Write-Report -Entries $entries
} catch [System.IO.IOException] {
    Write-Error $_.Exception.Message
} finally {
    Remove-Variable -Name entries -ErrorAction SilentlyContinue
}

switch -regex ($name) {
    '^ledger\s' { 'a ledger' }
    default { 'other' }
}

for ($i = 0; $i -lt 3; $i++) { $i * 2 }
while ($false) { break }
do { $i-- } until ($i -le 0)
& "$env:ProgramFiles\tool.exe" --version 2>&1 > $null
. ./helpers.ps1
[System.Math]::Round(2.5) | Out-Null
