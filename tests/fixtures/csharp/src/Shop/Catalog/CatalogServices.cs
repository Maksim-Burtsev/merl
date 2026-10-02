using Microsoft.Extensions.Options;
//                         ^ d: none
using Microsoft.EntityFrameworkCore.Migrations;
//                                  ^ d: none
using Shop.Catalog.Migrations;
//         ^ d: src/Shop/Catalog/CatalogServices.cs:10
//                 ^ d: src/Shop/Catalog/CatalogServices.cs:10
// No namespace of the project is `Microsoft`: the `using` names nothing here.

namespace Shop.Catalog.Migrations;

public class CatalogServices
{
    public IOptions<CatalogOptions> Options { get; } = null;

    private global::Shop.Pricing.Tariff? _tariff;
    //                   ^ d: src/Shop/Pricing/Pricing.cs:1
    //                           ^ d: src/Shop/Pricing/Pricing.cs:12
}
public class DupBase { }

public partial class Page
{
    public void Show() { }
}
