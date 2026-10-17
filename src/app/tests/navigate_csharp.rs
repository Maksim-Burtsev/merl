use super::*;

fn cs_app(tag: &str, files: &[(&str, &str)]) -> (PathBuf, App) {
    let (dir, mut a) = project_app(tag, files);
    a.external
        .insert(Kind::CSharp, (Vec::new(), Arc::new(Vec::new())));
    (dir, a)
}

const REDIRECT: &str = "public interface IRedirectService\n{\n    string ExtractRedirectUri(string url);\n}\n\npublic class RedirectService : IRedirectService\n{\n    public string ExtractRedirectUri(string url) => url;\n}\n";

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
        ),
        "A target-typed `new()` takes the type the method returns"
    );
    d_on(&mut a, "DeviceController.cs", "new { |Checked");
    assert!(
        matches!(shown(&mut a), Shown::Picker(..)),
        "An anonymous type declares its members where it stands: the search by name, as before"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

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
        a.message, "no definition for Text in the project (via _badgeIndicator: Label)",
        "a receiver of a type the project does not declare is the framework's: its member has no definition here, not the project's namesake"
    );
    d_on(&mut a, "BadgeView.cs", "_badgeIndicator.|Pulse");
    assert_eq!(
        shown(&mut a),
        jump(
            "Pulse \u{2192} LabelExtensions.Pulse (via _badgeIndicator: Label)",
            "LabelExtensions.cs:3"
        ),
        "an extension method the project declares for it is the answer"
    );
    d_on(&mut a, "BadgeView.cs", "this.|Text");
    assert_eq!(
        shown(&mut a),
        jump(
            "Text \u{2192} BadgeView.Text (via this: BadgeView)",
            "BadgeView.cs:5"
        )
    );
    d_on(&mut a, "BadgeView.cs", "this.|Opacity");
    assert_eq!(
        a.message, "no definition for Opacity in the project (via this: BadgeView)",
        "Not in the class, whose base is the framework's"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

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
        ),
        "a primary constructor's parameter"
    );
    d_on(&mut a, "Service.cs", "loaded.Users.|Delete");
    assert_eq!(
        shown(&mut a),
        jump(
            "Delete \u{2192} Repo.Delete (via loaded: Uow \u{2192} Users: UserRepo)",
            "Repos.cs:3"
        )
    );
    d_on(&mut a, "Service.cs", "item.|Delete");
    assert!(
        matches!(shown(&mut a), Shown::Picker(..)),
        "A type parameter proves nothing: the search by name, as before"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

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
    assert!(
        matches!(shown(&mut a), Shown::Picker(s, _) if s == "Order: by name, 2 declarations"),
        "a partial type may have a part a source generator writes: it proves no member is the framework's"
    );
    d_on(&mut a, "Tests.cs", "element.|Number");
    assert_eq!(
        shown(&mut a),
        jump(
            "Number \u{2192} Order.Number (by name, 1 match)",
            "Order.cs:3"
        ),
        "a type parameter is no type: it proves no member is the framework's"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

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
                "using Shop.Api;\nusing Shop.App;\nnamespace Shop.Web;\npublic class Page\n{\n    public Address Home() => new Address { Street = \"Main\" };\n}\n",
            ),
            (
                "Shop.App/Home.cs",
                "namespace Shop.App;\npublic class Home\n{\n    public Address At() => new Address { Street = \"Home\" };\n}\n",
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
    d_on(&mut a, "Shop.App/Home.cs", "new |Address");
    assert_eq!(
        shown(&mut a),
        jump("Address: by name, 1 match", "Shop.App/Address.cs:2"),
        "inside namespace Shop.App its own Address hides the referenced project's"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_constant_after_is_is_found_when_no_type_of_the_name_is_reached() {
    let (_dir, mut a) = cs_app(
        "cs-581",
        &[
            (
                "Gauge.cs",
                "namespace Shop;\n\npublic class Gauge\n{\n    public const int Max = 10;\n\n    public bool Full(int n) => n is Max;\n\n    public bool Typed(object o) => o is Max m;\n}\n",
            ),
            (
                "Limits.cs",
                "namespace Shop;\n\npublic class Limits\n{\n    public class Max { }\n}\n",
            ),
        ],
    );
    d_on(&mut a, "Gauge.cs", "n is |Max");
    assert_eq!(
        shown(&mut a),
        jump("Max \u{2192} Gauge.Max (via Gauge)", "Gauge.cs:5"),
        "a public type nested in another class is no type the walk by type reaches"
    );
    d_on(&mut a, "Gauge.cs", "o is |Max m");
    assert_eq!(
        shown(&mut a),
        jump("no definition for Max", "Gauge.cs:9"),
        "a declaration pattern keeps the lookup by type"
    );
}

const UNPROVEN: &[(&str, &str)] = &[
    (
        "Shop/Order.cs",
        "namespace Shop;\n\npublic class Order\n{\n    public int Id;\n    public int Total() => 1;\n}\n",
    ),
    (
        "Shop/Receipt.cs",
        "namespace Shop;\n\npublic class Receipt\n{\n    public int Total() => 2;\n}\n",
    ),
    (
        "Shop/Use.cs",
        "namespace Shop;\n\npublic class Use\n{\n    public Receipt Process(Order o) => new Receipt();\n    public int Run() => Process(new Order { Id = 1 }).Total();\n    public int Text() => new StringBuilder().Append(1).Length;\n}\n",
    ),
    (
        "Clock/Engine.cs",
        "namespace Clock;\n\npublic class Engine\n{\n    public void Start() { }\n}\n",
    ),
    (
        "Clock/Timer.cs",
        "namespace Clock;\n\npublic class Timer\n{\n    public void Start() { }\n}\n",
    ),
    (
        "Clock/UseT.cs",
        "namespace Clock;\n\npublic class UseT\n{\n    public void Go() { new System.Timers.Timer(1).Start(); }\n}\n",
    ),
    (
        "Pot/Kettle.cs",
        "namespace Pot;\n\npublic partial class Kettle\n{\n    static Kettle() { }\n    public Kettle(int a) => A = a;\n    public int A;\n}\n",
    ),
    (
        "Pot/Kettle.More.cs",
        "namespace Pot;\n\npublic partial class Kettle\n{\n    public Kettle(string b) { }\n}\n",
    ),
    (
        "Pot/Cup.cs",
        "namespace Pot;\n\npublic partial class Cup(int size)\n{\n    public int Size => size;\n}\n",
    ),
    (
        "Pot/Cup.More.cs",
        "namespace Pot;\n\npublic partial class Cup\n{\n    public Cup(string s) : this(s.Length) { }\n}\n",
    ),
    (
        "Pot/Mug.cs",
        "namespace Pot;\n\npublic partial class Mug\n{\n}\n",
    ),
    (
        "Pot/Mug.Gen.cs",
        "namespace Pot;\n\npublic partial class Mug<T>\n{\n    public Mug() { }\n}\n",
    ),
    (
        "Pot/UseK.cs",
        "namespace Pot;\n\npublic class UseK\n{\n    public object Fresh() => new Kettle();\n    public object Cup() => new Cup(1);\n    public object Mug() => new Mug();\n}\n",
    ),
    (
        "Nest/A.cs",
        "namespace Outer\n{\n    namespace Inner\n    {\n        public class Lid { }\n    }\n}\n",
    ),
    (
        "Nest/B.cs",
        "namespace Outer\n{\n    public class Lid { }\n}\n",
    ),
    (
        "Nest/UseL.cs",
        "namespace Outer.Inner;\n\npublic class UseL\n{\n    public Lid Get() => null;\n}\n",
    ),
    (
        "Dup/One.cs",
        "namespace Dup.One;\n\npublic class Tray { }\n",
    ),
    (
        "Dup/Two.cs",
        "namespace Dup.Two;\n\npublic class Tray { }\n",
    ),
];

#[test]
fn d_keeps_the_picker_where_csharp_s_choice_is_not_proven() {
    let (dir, mut a) = cs_app("cs-935-unproven", UNPROVEN);
    let mut places = |file: &str, code: &str| {
        d_on(&mut a, file, code);
        match shown(&mut a) {
            Shown::Picker(_, rows) => rows.into_iter().map(|(_, _, place)| place).collect(),
            Shown::Jump(status, place) => vec![format!("jump {place} ({status})")],
        }
    };
    assert_eq!(
        places("Shop/Use.cs", "Id = 1 }).|Total"),
        ["Shop/Order.cs:6", "Shop/Receipt.cs:5"],
        "the initializer is not the receiver"
    );
    assert_eq!(
        places("Clock/UseT.cs", "Timer(1).|Start"),
        ["Clock/Engine.cs:5", "Clock/Timer.cs:5"],
        "System.Timers.Timer is not the project's Clock.Timer"
    );
    assert_eq!(
        places("Pot/UseK.cs", "new |Kettle()"),
        ["Pot/Kettle.More.cs:3", "Pot/Kettle.cs:3"],
        "no instance constructor takes no argument; the static one is never called"
    );
    assert_eq!(
        places("Pot/UseK.cs", "new |Cup(1)"),
        ["Pot/Cup.More.cs:3", "Pot/Cup.cs:3"],
        "the primary constructor is the one called"
    );
    assert_eq!(
        places("Pot/UseK.cs", "new |Mug()"),
        ["Pot/Mug.Gen.cs:3", "Pot/Mug.cs:3"],
        "Mug and Mug<T> are two types"
    );
    assert_eq!(
        places("Nest/UseL.cs", "public |Lid"),
        ["Nest/A.cs:5", "Nest/B.cs:3"],
        "the nested block namespace is Outer.Inner, not Inner"
    );
    assert_eq!(
        places("Dup/One.cs", "class |Tray"),
        ["Dup/Two.cs:3"],
        "on its own declaration the namesake stays offered"
    );
    d_on(&mut a, "Shop/Use.cs", "new StringBuilder().|Append");
    assert_eq!(
        a.message,
        "no definition for Append in the project (via new StringBuilder)"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
