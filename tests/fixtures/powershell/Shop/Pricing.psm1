# The shop of #307: every d case carries its answer in a comment under it.
# A comment that reads like code declares nothing:
# function Measure-Weight {

<#
.SYNOPSIS
Prices of the shop. Comment-based help holds calls and assignments:
.EXAMPLE
function Measure-Weight { 1 }
$RateCap = 5
#>

enum Status {
    Active
    Closed = 2
}

class Tariff {
    [decimal] $Gross
    hidden [int]$Count = 0

    Tariff([decimal] $gross) {
        $this.Gross = $gross
    }

    [decimal] Rate() {
        return 1
    }

    [string] Describe() {
        return "tariff $($this.Count)"
    }
}

class Coupon : Tariff {
#              ^ d: Shop/Pricing.psm1:18
    static [Coupon] Parse([string] $s) {
        return [Coupon]::new(1)
    }

    [decimal] Rate() {
        return 2
    }
}

$script:RateCap = 100
$global:Currency = 'EUR'
[string]$Symbol = 'E'
$Ledger = 0
$Ledger += 1

$Template = @"
function Measure-Weight {
`$RateCap = 1
"@

$Literal = @'
function Format-Label {
'@

function Get-Discount {
    param(
        [Parameter(Mandatory)][decimal]$Total,
        [string]$Code = 'none'
    )
    if ($Total -eq $script:RateCap) { return 0 }
    #    ^ d: Shop/Pricing.psm1:63
    # status: Total: local
    #                      ^ d: Shop/Pricing.psm1:46
    # status: RateCap: by name, 1 match
    $Total - $Code.Length
    #         ^ d: Shop/Pricing.psm1:64
}

function Get-Gross($Net) {
    $Net * 2
    #^ d: Shop/Pricing.psm1:75
    # status: Net: local
}

function global:Measure-Weight {
    param([int]$Grams)
    $Grams / 1000
    #^ d: Shop/Pricing.psm1:82
}

function script:Format-Label {
    "label $Total"
    #       ^ d: none
}

filter Select-Active {
    if ($_.Status -eq [Status]::Active) { $_ }
    #                           ^ d: Shop/Pricing.psm1:14
}

Set-Alias -Name gr -Value Get-Gross
New-Alias wt Measure-Weight

Export-ModuleMember -Function Get-Discount, Get-Gross, Measure-Weight, Select-Active
#                             ^ d: Shop/Pricing.psm1:61
#                                                      ^ d: Shop/Pricing.psm1:81

# An attribute in front of a typed assignment holds a bracket of its own.
[ValidateRange(1, 9)][int]$Retries = 3
