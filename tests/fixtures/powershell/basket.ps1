using module ./Shop/Pricing.psm1
#                   ^ d: Shop/Pricing.psm1:1
param(
    [string]$Region = 'eu'
)

. $PSScriptRoot/Shop/Courier.ps1
#                    ^ d: Shop/Courier.ps1:1
Import-Module ./Shop/Pricing.psm1
#                    ^ d: Shop/Pricing.psm1:1

$cut = get-discount -Total 42
#      ^ d: Shop/Pricing.psm1:61
# status: get-discount: by name, 1 match
#                    ^ d: none
$grams = Measure-Weight -Grams 500
#        ^ d: Shop/Pricing.psm1:81
$label = Format-Label
#        ^ d: Shop/Pricing.psm1:87
$active = $orders | Select-Active
#                   ^ d: Shop/Pricing.psm1:92
$net = gr 5; $kg = wt 300
#      ^ d: Shop/Pricing.psm1:97
#                  ^ d: Shop/Pricing.psm1:98

"$Symbol $global:Currency for $Region"
# ^ d: Shop/Pricing.psm1:48
#                ^ d: Shop/Pricing.psm1:47
#                              ^ d: basket.ps1:4
# status: Region: local
"$Ledger"
# ^ d: picker Shop/Pricing.psm1:49, Shop/Pricing.psm1:50

$tariff = [Tariff]::new(5)
#          ^ d: picker Shop/Pricing.psm1:18, Shop/Pricing.psm1:22
New-Object Tariff -ArgumentList 5
#          ^ d: Shop/Pricing.psm1:18
$status = [Status]::Closed
#          ^ d: Shop/Pricing.psm1:13
#                   ^ d: Shop/Pricing.psm1:15

# Uses that declare nothing: a call, a hashtable key, a property and an element write, a
# comparison, splatting.
 Send-Parcel
$params = @{
    Courier = 'dhl'
}
$parcel.Courier = 'ups'
$rates['Courier'] = 1
if ($Courier -eq 'post') { Send-Parcel @params }
#    ^ d: Shop/Courier.ps1:7
#                          ^ d: Shop/Courier.ps1:2
#                                       ^ d: basket.ps1:45

# A backtick continues a line and hides nothing after it.
$taxed = Get-Tax `
    -Amount 10
$again = Get-Tax 3
#        ^ d: basket.ps1:61

function Get-Tax($Amount) { $Amount / 5 }

"tries: $Retries"
#        ^ d: Shop/Pricing.psm1:105
"$env:ShopRoot $status"
#     ^ d: Shop/Pricing.psm1:137
#               ^ d: basket.ps1:38
$units = [Unit]::Kilo + [Unit]::Gram
#                ^ d: Shop/Pricing.psm1:124
#                               ^ d: Shop/Pricing.psm1:123
$cut2 = [Coupon]::Parse('x')
#                 ^ d: Shop/Pricing.psm1:37
"$Code"
# ^ d: none
