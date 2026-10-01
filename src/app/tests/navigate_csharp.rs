//! #352: `d` in C# reads the type a receiver or an object initializer is written with.

use super::*;

fn cs_app(tag: &str, files: &[(&str, &str)]) -> (PathBuf, App) {
    let (dir, mut a) = project_app(tag, files);
    a.external
        .insert(Kind::CSharp, (Vec::new(), Arc::new(Vec::new())));
    (dir, a)
}

const REDIRECT: &str = "public interface IRedirectService\n{\n    string ExtractRedirectUri(string url);\n}\n\npublic class RedirectService : IRedirectService\n{\n    public string ExtractRedirectUri(string url) => url;\n}\n";

/// The repros of the issue: a local constructed with `new`, and an initializer's member.
#[test]
fn a_typed_local_and_an_initializer_member_jump_to_their_type() {
    let (dir, mut a) = cs_app(
        "cs-typed",
        &[
            ("RedirectService.cs", REDIRECT),
            (
                "RedirectTests.cs",
                "public class RedirectTests\n{\n    public void Extracts()\n    {\n        var service = new RedirectService();\n        Assert.AreEqual(\"\", service.ExtractRedirectUri(\"/connect\"));\n    }\n}\n",
            ),
            (
                "ScopeViewModel.cs",
                "public class ScopeViewModel\n{\n    public string Value { get; set; }\n    public bool Checked { get; set; }\n}\n",
            ),
            (
                "ToggleButton.cs",
                "public class ToggleButton\n{\n    public bool Checked { get; set; }\n}\n",
            ),
            (
                "DeviceController.cs",
                "public class DeviceController\n{\n    private ScopeViewModel CreateScopeViewModel(bool check)\n    {\n        return new ScopeViewModel\n        {\n            Value = \"openid\",\n            Checked = check\n        };\n    }\n\n    private ScopeViewModel Target(bool check)\n    {\n        return new()\n        {\n            Checked = !check\n        };\n    }\n\n    private object Anonymous(bool check) => new { Checked = check };\n}\n",
            ),
        ],
    );
    d_on(&mut a, "RedirectTests.cs", "service.|ExtractRedirectUri");
    assert_eq!(
        shown(&mut a),
        jump(
            "ExtractRedirectUri \u{2192} RedirectService.ExtractRedirectUri (via service: RedirectService)",
            "RedirectService.cs:8"
        )
    );
    d_on(
        &mut a,
        "DeviceController.cs",
        "^            |Checked = check",
    );
    assert_eq!(
        shown(&mut a),
        jump(
            "Checked \u{2192} ScopeViewModel.Checked (via new ScopeViewModel)",
            "ScopeViewModel.cs:4"
        )
    );
    // A target-typed `new()` takes the type the method returns.
    d_on(
        &mut a,
        "DeviceController.cs",
        "^            |Checked = !check",
    );
    assert_eq!(
        shown(&mut a),
        jump(
            "Checked \u{2192} ScopeViewModel.Checked (via new ScopeViewModel)",
            "ScopeViewModel.cs:4"
        )
    );
    // An anonymous type declares its members where it stands: the search by name, as before.
    d_on(&mut a, "DeviceController.cs", "new { |Checked");
    assert!(matches!(shown(&mut a), Shown::Picker(..)));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A receiver of a type the project does not declare is the framework's: its member has no
/// definition here, not the project's namesake (eShop's `_badgeIndicator.Text` on a MAUI `Label`).
/// An extension method the project declares for it is the answer.
#[test]
fn a_member_of_a_framework_type_has_no_definition_but_its_extension_method() {
    let (dir, mut a) = cs_app(
        "cs-outside",
        &[
            (
                "BadgeView.cs",
                "public class BadgeView : ContentView\n{\n    private readonly Label _badgeIndicator;\n\n    public string Text { get; set; }\n\n    void Update()\n    {\n        _badgeIndicator.Text = Text;\n        _badgeIndicator.Pulse();\n        this.Text = \"\";\n        this.Opacity = 1;\n    }\n}\n",
            ),
            (
                "LabelExtensions.cs",
                "public static class LabelExtensions\n{\n    public static void Pulse(this Label label) { }\n}\n",
            ),
        ],
    );
    d_on(&mut a, "BadgeView.cs", "_badgeIndicator.|Text");
    assert_eq!(
        a.message,
        "no definition for Text in the project (via _badgeIndicator: Label)"
    );
    d_on(&mut a, "BadgeView.cs", "_badgeIndicator.|Pulse");
    assert_eq!(
        shown(&mut a),
        jump(
            "Pulse \u{2192} LabelExtensions.Pulse (via _badgeIndicator: Label)",
            "LabelExtensions.cs:3"
        )
    );
    d_on(&mut a, "BadgeView.cs", "this.|Text");
    assert_eq!(
        shown(&mut a),
        jump(
            "Text \u{2192} BadgeView.Text (via this: BadgeView)",
            "BadgeView.cs:5"
        )
    );
    // Not in the class, whose base is the framework's.
    d_on(&mut a, "BadgeView.cs", "this.|Opacity");
    assert_eq!(
        a.message,
        "no definition for Opacity in the project (via this: BadgeView)"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A chain of fields and properties, a member inherited from a base, a call's awaited return
/// type and a primary constructor's parameter.
#[test]
fn a_chain_a_base_and_an_awaited_call_are_followed() {
    let (dir, mut a) = cs_app(
        "cs-chain",
        &[
            (
                "Repos.cs",
                "public abstract class Repo\n{\n    public void Delete(int id) { }\n}\n\npublic class UserRepo : Repo\n{\n}\n\npublic class AuditLog\n{\n    public void Delete(int id) { }\n}\n\npublic class Uow\n{\n    public UserRepo Users { get; }\n    public AuditLog Audit => new AuditLog();\n}\n",
            ),
            (
                "Service.cs",
                "public class Service(Uow uow)\n{\n    private readonly Uow _uow = uow;\n\n    public async Task<Uow> LoadAsync() => _uow;\n\n    public async Task Run()\n    {\n        _uow.Users.Delete(1);\n        uow.Audit.Delete(2);\n        var loaded = await LoadAsync();\n        loaded.Users.Delete(3);\n    }\n\n    public void Generic<T>(T item) where T : Repo\n    {\n        item.Delete(4);\n    }\n}\n",
            ),
        ],
    );
    let base = jump(
        "Delete \u{2192} Repo.Delete (via _uow: Uow \u{2192} Users: UserRepo)",
        "Repos.cs:3",
    );
    d_on(&mut a, "Service.cs", "_uow.Users.|Delete");
    assert_eq!(shown(&mut a), base);
    d_on(&mut a, "Service.cs", "uow.Audit.|Delete");
    assert_eq!(
        shown(&mut a),
        jump(
            "Delete \u{2192} AuditLog.Delete (via uow: Uow \u{2192} Audit: AuditLog)",
            "Repos.cs:12"
        )
    );
    d_on(&mut a, "Service.cs", "loaded.Users.|Delete");
    assert_eq!(
        shown(&mut a),
        jump(
            "Delete \u{2192} Repo.Delete (via loaded: Uow \u{2192} Users: UserRepo)",
            "Repos.cs:3"
        )
    );
    // A type parameter proves nothing: the search by name, as before.
    d_on(&mut a, "Service.cs", "item.|Delete");
    assert!(matches!(shown(&mut a), Shown::Picker(..)));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A partial type may have a part a source generator writes, and a type parameter is no type:
/// neither proves a member is the framework's, and the search by name answers, as before.
#[test]
fn a_partial_type_or_a_type_parameter_leaves_the_member_to_the_name() {
    let (dir, mut a) = cs_app(
        "cs-partial",
        &[
            (
                "OrderViewModel.cs",
                "public partial class OrderViewModel : ObservableObject\n{\n    [ObservableProperty] private Order _order;\n}\n",
            ),
            (
                "Order.cs",
                "public class Order\n{\n    public int Number { get; set; }\n}\n",
            ),
            (
                "Invoice.cs",
                "public class Invoice\n{\n    public Order Order { get; set; }\n}\n",
            ),
            (
                "Tests.cs",
                "public class Tests\n{\n    public void Check(OrderViewModel model)\n    {\n        var o = model.Order;\n    }\n\n    public static Task<TElement> Hash<TElement>(this TElement element) where TElement : Order\n    {\n        return element.Number;\n    }\n}\n",
            ),
        ],
    );
    d_on(&mut a, "Tests.cs", "model.|Order");
    assert!(matches!(shown(&mut a), Shown::Picker(s, _) if s == "Order: by name, 2 declarations"));
    d_on(&mut a, "Tests.cs", "element.|Number");
    assert_eq!(
        shown(&mut a),
        jump(
            "Number \u{2192} Order.Number (by name, 1 match)",
            "Order.cs:3"
        )
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// #349, the repro of the issue: a file of `Shop.App` sees its own project, not `Shop.Api`'s
/// namesake, until `Shop.App.csproj` references `Shop.Api`.
#[test]
fn a_file_sees_its_own_project_and_the_ones_it_references() {
    let files = |app: &'static str| {
        [
            (
                "Shop.Api/Shop.Api.csproj",
                r#"<Project Sdk="Microsoft.NET.Sdk"></Project>"#,
            ),
            ("Shop.App/Shop.App.csproj", app),
            (
                "Shop.Api/Address.cs",
                "namespace Shop.Api;\npublic class Address\n{\n    public string Street { get; set; }\n}\n",
            ),
            (
                "Shop.App/Address.cs",
                "namespace Shop.App;\npublic class Address\n{\n    public string Street { get; set; }\n}\n",
            ),
            (
                "Shop.App/Page.cs",
                "namespace Shop.App;\npublic class Page\n{\n    public Address Home() => new Address { Street = \"Main\" };\n}\n",
            ),
        ]
    };
    let (dir, mut a) = cs_app(
        "cs-projects",
        &files(r#"<Project Sdk="Microsoft.NET.Sdk"></Project>"#),
    );
    d_on(&mut a, "Shop.App/Page.cs", "new |Address");
    assert_eq!(
        shown(&mut a),
        jump("Address: by name, 1 match", "Shop.App/Address.cs:2")
    );
    std::fs::remove_dir_all(&dir).unwrap();
    let (dir, mut a) = cs_app(
        "cs-projects-referenced",
        &files(
            r#"<Project Sdk="Microsoft.NET.Sdk"><ItemGroup><ProjectReference Include="..\Shop.Api\Shop.Api.csproj" /></ItemGroup></Project>"#,
        ),
    );
    d_on(&mut a, "Shop.App/Page.cs", "new |Address");
    assert!(
        matches!(shown(&mut a), Shown::Picker(s, rows) if s.contains("2 declarations") && rows.len() == 2)
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
