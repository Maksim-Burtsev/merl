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
        ["jump Shop/Receipt.cs:5 (Total → Receipt.Total (via Process(): Receipt))"],
        "the initializer is not the receiver, the Receipt that Process returns is"
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

const VALIDATED: &[(&str, &str)] = &[
    (
        "Commands/Order.cs",
        "namespace Shop.Commands;\n\npublic class Order\n{\n    public string Street { get; set; }\n}\n",
    ),
    (
        "Commands/Wrapped.cs",
        "namespace Shop.Commands;\n\npublic class Wrapped<T, R>\n{\n    public int Id { get; }\n}\n",
    ),
    (
        "Other/Address.cs",
        "namespace Shop.Other;\n\npublic class Address\n{\n    public string Street { get; set; }\n    public int Id { get; }\n}\n",
    ),
    (
        "Validations/OrderValidator.cs",
        "using Shop.Commands;\n\nnamespace Shop.Validations;\n\npublic class OrderValidator : AbstractValidator<Order>\n{\n    public OrderValidator()\n    {\n        RuleFor(command => command.Street).NotEmpty();\n    }\n}\n\npublic class WrappedValidator : AbstractValidator<Wrapped<Order, bool>>\n{\n    public WrappedValidator()\n    {\n        RuleFor(command => command.Id).NotEmpty();\n    }\n}\n",
    ),
];

#[test]
fn a_validator_s_rule_lambda_takes_the_type_its_base_validates() {
    let (dir, mut a) = cs_app("cs-1298-validated", VALIDATED);
    d_on(
        &mut a,
        "Validations/OrderValidator.cs",
        "command => command.|Street",
    );
    assert_eq!(
        shown(&mut a),
        jump(
            "Street \u{2192} Order.Street (via command: Order)",
            "Commands/Order.cs:5"
        )
    );
    d_on(
        &mut a,
        "Validations/OrderValidator.cs",
        "command => command.|Id",
    );
    assert_eq!(
        shown(&mut a),
        jump(
            "Id \u{2192} Wrapped.Id (via command: Wrapped)",
            "Commands/Wrapped.cs:5"
        )
    );
    std::fs::remove_dir_all(&dir).unwrap();
    let mut files = VALIDATED.to_vec();
    files.push((
        "Validations/AbstractValidator.cs",
        "namespace Shop.Validations;\n\npublic class AbstractValidator<T>\n{\n    public void RuleFor(Func<Address, object> f) { }\n}\n",
    ));
    let (dir, mut a) = cs_app("cs-1298-namesake", &files);
    d_on(
        &mut a,
        "Validations/OrderValidator.cs",
        "command => command.|Street",
    );
    assert!(
        matches!(shown(&mut a), Shown::Picker(..)),
        "the project's own AbstractValidator is not FluentValidation's"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

const ALIASED: &[(&str, &str)] = &[
    (
        "Queries/Card.cs",
        "namespace Shop.Queries;\n\npublic record Card\n{\n    public int Id { get; init; }\n}\n",
    ),
    (
        "Domain/Card.cs",
        "namespace Shop.Domain;\n\npublic class Card\n{\n}\n",
    ),
    (
        "Apis/CardsApi.cs",
        "using Card = Shop.Queries.Card;\n\npublic static class CardsApi\n{\n    public static Task<List<Card>> GetCardsAsync() => null!;\n}\n",
    ),
    (
        "Domain/Use.cs",
        "using Card = Shop.Queries.Card;\n\nnamespace Shop.Domain;\n\npublic static class Use\n{\n    public static Card Make() => null!;\n}\n",
    ),
];

#[test]
fn a_using_alias_of_the_file_names_its_target() {
    let (dir, mut a) = cs_app("cs-1298-aliased", ALIASED);
    d_on(&mut a, "Apis/CardsApi.cs", "List<|Card>");
    assert_eq!(
        shown(&mut a),
        jump(
            "Card: via using Card = Shop.Queries.Card",
            "Queries/Card.cs:3"
        )
    );
    d_on(&mut a, "Domain/Use.cs", "static |Card Make");
    assert!(
        matches!(shown(&mut a), Shown::Picker(..)),
        "Shop.Domain's own Card comes before the file's alias"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

const BASKET: &[(&str, &str)] = &[
    (
        "Item.cs",
        "public class Item\n{\n    public decimal Price { get; }\n}\n",
    ),
    (
        "Receipt.cs",
        "public class Receipt\n{\n    public decimal Price { get; }\n}\n",
    ),
    (
        "Basket.cs",
        "using System.Collections.Generic;\n\npublic class Basket : List<Item>\n{\n}\n",
    ),
    (
        "BasketExtensions.cs",
        "public static class BasketExtensions\n{\n    public static Receipt Find(this Basket b, Predicate<Item> p) => null!;\n}\n",
    ),
    (
        "Till.cs",
        "public class Till\n{\n    public decimal Ring(Basket basket)\n    {\n        return basket.Find(i => i.Price > 0).Price;\n    }\n}\n",
    ),
];

#[test]
fn an_extension_method_does_not_shadow_a_method_a_base_outside_the_project_may_declare() {
    let (dir, mut a) = cs_app("cs-instance-first", BASKET);
    d_on(&mut a, "Till.cs", "> 0).|Price");
    let got = shown(&mut a);
    assert!(
        matches!(got, Shown::Picker(..)),
        "List<Item>.Find, an instance method, beats the extension Find of Basket: {got:?}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn an_extension_method_does_not_answer_a_call_a_base_outside_the_project_may_declare() {
    let (dir, mut a) = cs_app("cs-instance-first-member", BASKET);
    d_on(&mut a, "Till.cs", "basket.|Find");
    let got = shown(&mut a);
    assert_eq!(
        got,
        jump(
            "no definition for Find in the project (via basket: Basket)",
            "Till.cs:5"
        ),
        "List<Item>.Find, an instance method, beats the extension Find of Basket"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

const COALESCED: &[(&str, &str)] = &[
    (
        "Order.cs",
        "public class Order\n{\n    public decimal Total { get; }\n}\n",
    ),
    (
        "Invoice.cs",
        "public class Invoice\n{\n    public decimal Total { get; }\n}\n",
    ),
    (
        "Cache.cs",
        "public class Cache\n{\n    public Order Find(int id) => null!;\n}\n",
    ),
    (
        "Repo.cs",
        "public class Repo\n{\n    public Order Load(int id) => null!;\n}\n",
    ),
    (
        "Checkout.cs",
        "public class Checkout\n{\n    private readonly Cache cache = new Cache();\n    private readonly Repo repo = new Repo();\n\n    public decimal Sum(int id)\n    {\n        var order = cache.Find(id) ?? repo.Load(id);\n        return order.Total;\n    }\n}\n",
    ),
];

#[test]
fn a_coalesce_of_two_values_keeps_the_type_its_right_hand_chain_gives() {
    let (dir, mut a) = cs_app("cs-coalesced", COALESCED);
    d_on(&mut a, "Checkout.cs", "order.|Total");
    let got = shown(&mut a);
    assert!(
        matches!(&got, Shown::Jump(_, place) if place == "Order.cs:3"),
        "`cache.Find(id) ?? repo.Load(id)` is an Order: {got:?}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

const SLICED: &[(&str, &str)] = &[
    (
        "Cup.cs",
        "public class Cup\n{\n    public int Sip() => 0;\n}\n",
    ),
    (
        "Mug.cs",
        "public class Mug\n{\n    public int Sip() => 0;\n}\n",
    ),
    (
        "Tray.cs",
        "using System.Collections.Immutable;\n\npublic class Tray\n{\n    public int Pour(ImmutableArray<Cup> cups, int a, int b)\n    {\n        var rest = cups[1..];\n        var middle = cups[a..b];\n        var one = cups[a];\n        return rest.Sip() + middle.Sip() + one.Sip();\n    }\n}\n",
    ),
];

#[test]
fn a_range_slice_is_the_collection_not_one_element() {
    let (dir, mut a) = cs_app("cs-sliced", SLICED);
    for probe in ["rest.|Sip", "middle.|Sip"] {
        d_on(&mut a, "Tray.cs", probe);
        let got = shown(&mut a);
        assert!(
            matches!(got, Shown::Picker(..)),
            "{probe}: a slice is the collection: {got:?}"
        );
    }
    d_on(&mut a, "Tray.cs", "one.|Sip");
    let got = shown(&mut a);
    assert!(
        matches!(&got, Shown::Jump(_, place) if place == "Cup.cs:3"),
        "an index is one element: {got:?}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

const AWAITED: &[(&str, &str)] = &[
    (
        "Order.cs",
        "public class Order\n{\n    public int Length { get; }\n}\n",
    ),
    (
        "Ribbon.cs",
        "public class Ribbon\n{\n    public int Length { get; }\n}\n",
    ),
    (
        "Repo.cs",
        "using System.Threading.Tasks;\n\npublic class Repo\n{\n    public Task<Order> GetOrderAsync() => null!;\n}\n",
    ),
    (
        "Label.cs",
        "using System.Threading.Tasks;\n\npublic class Label\n{\n    private readonly Repo repo = new Repo();\n\n    public async Task<int> Print(string prefix)\n    {\n        var text = prefix + (await repo.GetOrderAsync());\n        var order = (await repo.GetOrderAsync());\n        return text.Length + order.Length;\n    }\n}\n",
    ),
];

#[test]
fn only_an_await_that_opens_the_expression_heads_its_chain() {
    let (dir, mut a) = cs_app("cs-awaited-head", AWAITED);
    d_on(&mut a, "Label.cs", "text.|Length");
    let got = shown(&mut a);
    assert!(
        matches!(got, Shown::Picker(..)),
        "`prefix + (await …)` is no Order: {got:?}"
    );
    d_on(&mut a, "Label.cs", "order.|Length");
    let got = shown(&mut a);
    assert!(
        matches!(&got, Shown::Jump(_, place) if place == "Order.cs:3"),
        "a grouped await alone is the awaited Order: {got:?}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

const SELECTED: &[(&str, &str)] = &[
    (
        "Order.cs",
        "public class Order\n{\n    public string Note { get; }\n    public Buyer Buyer { get; }\n}\n",
    ),
    (
        "Buyer.cs",
        "public class Buyer\n{\n    public string Name { get; }\n}\n",
    ),
    (
        "Seller.cs",
        "public class Seller\n{\n    public string Name { get; }\n}\n",
    ),
    (
        "Report.cs",
        "using System.Collections.Generic;\nusing System.Linq;\n\npublic class Shop\n{\n    public List<Order> Orders { get; } = new();\n}\n\npublic class Report\n{\n    public void Show(Shop shop)\n    {\n        var labels = shop.Orders.Select(o => o.Note + o.Buyer);\n        foreach (var label in labels)\n        {\n            _ = label.Name;\n        }\n        var buyers = shop.Orders.Select(o => o.Buyer);\n        foreach (var buyer in buyers)\n        {\n            _ = buyer.Name;\n        }\n    }\n}\n",
    ),
];

#[test]
fn select_reads_a_lambda_body_only_when_it_is_one_member_chain() {
    let (dir, mut a) = cs_app("cs-select-body", SELECTED);
    d_on(&mut a, "Report.cs", "label.|Name");
    let got = shown(&mut a);
    assert!(
        matches!(got, Shown::Picker(..)),
        "`o.Note + o.Buyer` is no Buyer: {got:?}"
    );
    d_on(&mut a, "Report.cs", "buyer.|Name");
    let got = shown(&mut a);
    assert!(
        matches!(&got, Shown::Jump(_, place) if place == "Buyer.cs:3"),
        "`o => o.Buyer` makes Buyers: {got:?}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

const INHERITED: &[(&str, &str)] = &[
    (
        "UI/ControlCollection.cs",
        "namespace MyApp.UI;\n\npublic class ControlCollection\n{\n}\n",
    ),
    (
        "Other/ControlCollection.cs",
        "namespace MyApp.Other;\n\npublic class ControlCollection\n{\n}\n",
    ),
    (
        "Views/Box.cs",
        "namespace MyApp.Views;\n\npublic class Box\n{\n}\n",
    ),
    (
        "Views/Panel2.cs",
        "using ControlCollection = MyApp.UI.ControlCollection;\n\nnamespace MyApp.Views;\n\npublic class Panel2 : System.Windows.Forms.Control\n{\n    ControlCollection outer;\n}\n\npublic class Panel3 : Box\n{\n    ControlCollection inner;\n}\n",
    ),
];

#[test]
fn a_using_alias_answers_only_when_no_base_outside_the_project_may_nest_the_type() {
    let (dir, mut a) = cs_app("cs-alias-inherited", INHERITED);
    d_on(&mut a, "Views/Panel2.cs", "|ControlCollection outer");
    let got = shown(&mut a);
    assert!(
        !matches!(&got, Shown::Jump(status, _) if status.contains("using ControlCollection")),
        "Control's nested ControlCollection comes before the file's alias: {got:?}"
    );
    d_on(&mut a, "Views/Panel2.cs", "|ControlCollection inner");
    assert_eq!(
        shown(&mut a),
        jump(
            "ControlCollection: via using ControlCollection = MyApp.UI.ControlCollection",
            "UI/ControlCollection.cs:3"
        )
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
