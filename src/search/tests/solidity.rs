use super::*;

fn declares(line: &str, word: &str) -> bool {
    let lines = ["contract Shop {", line];
    Regex::new(&solidity_patterns(word).join("|"))
        .unwrap()
        .is_match(line)
        && declares_where(Kind::Solidity, Path::new("a.sol"), word, 2, line, || &lines)
}

#[test]
fn solidity_declaration_forms() {
    for (line, word) in [
        ("contract ShopToken is ERC20, Ownable {", "ShopToken"),
        ("abstract contract Base {", "Base"),
        ("interface IERC20 {", "IERC20"),
        ("library SafeMath {", "SafeMath"),
        (
            "    function mint(address to, uint256 amount) external onlyOwner {",
            "mint",
        ),
        (
            "    function transfer(address to, uint256 amount) external returns (bool);",
            "transfer",
        ),
        ("function free(uint256 a) pure returns (uint256) {", "free"),
        (
            "    modifier withinSupply(uint256 amount) {",
            "withinSupply",
        ),
        ("    modifier onlyOwner {", "onlyOwner"),
        (
            "    event Minted(address indexed to, uint256 amount);",
            "Minted",
        ),
        (
            "    error SupplyExceeded(uint256 requested);",
            "SupplyExceeded",
        ),
        ("    struct Order {", "Order"),
        ("    enum Side { Buy, Sell }", "Side"),
        ("    enum Side { Buy, Sell }", "Buy"),
        ("    enum Side { Buy, Sell }", "Sell"),
        ("type Price is uint128;", "Price"),
        ("    uint256 public totalSupply;", "totalSupply"),
        (
            "    mapping(address => uint256) private _balances;",
            "_balances",
        ),
        (
            "    mapping(address => mapping(address => uint256)) public allowance;",
            "allowance",
        ),
        (
            "    uint256 public constant MAX_SUPPLY = 1_000_000e18;",
            "MAX_SUPPLY",
        ),
        ("    address immutable owner;", "owner"),
        ("    address payable public treasury;", "treasury"),
        ("uint256 constant LIMIT = 10;", "LIMIT"),
        ("    Order[] public orders;", "orders"),
        ("    IERC20 public token;", "token"),
        ("    Lib.Order internal last;", "last"),
        (
            "    bytes32 public constant override ROLE = keccak256(\"R\");",
            "ROLE",
        ),
        ("        uint256 fee = amount / 100;", "fee"),
        ("        uint256 price;", "price"),
        ("        Order storage o = orders[0];", "o"),
    ] {
        assert!(declares(line, word), "{line}: {word}");
    }
}

#[test]
fn solidity_refusals() {
    for (line, word) in [
        ("        _mint(to, amount);", "_mint"),
        ("        emit Minted(to, amount);", "Minted"),
        ("        revert SupplyExceeded(amount);", "SupplyExceeded"),
        (
            "    function mint(address to, uint256 amount) external onlyOwner withinSupply(amount) {",
            "withinSupply",
        ),
        (
            "    function mint(address to, uint256 amount) external onlyOwner {",
            "onlyOwner",
        ),
        ("contract ShopToken is ERC20, Ownable {", "ERC20"),
        ("    using SafeMath for uint256;", "SafeMath"),
        ("    using SafeMath for uint256;", "uint256"),
        (
            "    function transfer(address to) external returns (bool ok);",
            "to",
        ),
        (
            "    function transfer(address to) external returns (bool ok);",
            "ok",
        ),
        ("    constructor() ERC20(\"Shop\", \"SHOP\") {}", "ERC20"),
        ("import {ERC20} from \"./ERC20.sol\";", "ERC20"),
        ("import {ERC20 as Base} from \"./ERC20.sol\";", "Base"),
        ("        return amount;", "amount"),
        ("        delete orders;", "orders"),
        ("        else total = 1;", "total"),
        ("        total = amount;", "total"),
        ("        if (total == amount) {", "amount"),
        ("        amount,", "amount"),
        ("    // function ghost() {", "ghost"),
    ] {
        assert!(!declares(line, word), "{line}: {word}");
    }
}

#[test]
fn a_solidity_enum_value_declares_directly_inside_its_enum() {
    let lines = [
        "contract Shop {",
        "    enum Zone {",
        "        Local,",
        "        Abroad",
        "    }",
        "    function f() {",
        "        g(",
        "            Local",
        "        );",
        "    }",
        "}",
    ];
    let at = |line: usize, word: &str| {
        declares_where(
            Kind::Solidity,
            Path::new("a.sol"),
            word,
            line,
            lines[line - 1],
            || &lines,
        )
    };
    assert!(at(3, "Local"));
    assert!(at(4, "Abroad"));
    assert!(!at(8, "Local"));
}

#[test]
fn solidity_symbols() {
    let sol = |line| one(Kind::Solidity, line);
    for (line, name) in [
        ("contract ShopToken is ERC20 {", "ShopToken"),
        ("abstract contract Base {", "Base"),
        ("library SafeMath {", "SafeMath"),
        ("interface IERC20 {", "IERC20"),
        ("    function mint(address to) external {", "mint"),
        ("    modifier onlyOwner {", "onlyOwner"),
        ("    event Minted(address indexed to);", "Minted"),
        (
            "    error SupplyExceeded(uint256 requested);",
            "SupplyExceeded",
        ),
        ("    struct Order {", "Order"),
        ("    enum Side { Buy, Sell }", "Side"),
        ("type Price is uint128;", "Price"),
    ] {
        assert_eq!(sol(line).as_deref(), Some(name), "{line}");
    }
    for line in [
        "    uint256 public totalSupply;",
        "    uint256 public constant MAX = 1;",
        "        emit Minted(to);",
    ] {
        assert_eq!(sol(line), None, "{line}");
    }
}

#[test]
fn solidity_literals_hide_declarations() {
    let text = "\
/**
 * contract Shadow {
 */
contract Real {
    /* event Ghost(uint256 id);
       error Gone(); */
    string s = \"/* not a comment\";
    uint256 x;
}
";
    let hidden: Vec<usize> = literal_lines(Kind::Solidity, text)
        .iter()
        .enumerate()
        .filter(|(_, h)| **h)
        .map(|(i, _)| i + 1)
        .collect();
    assert_eq!(hidden, [2, 3, 6]);
}

#[test]
fn solidity_imports_bind_names_and_qualifiers() {
    let text = "\
import {ERC20} from \"@openzeppelin/contracts/token/ERC20/ERC20.sol\";
import {ERC20 as Base, IERC20} from './IERC20.sol';
import {
    Ownable
} from \"./Ownable.sol\";
import \"./Token.sol\" as T;
import * as U from \"./Util.sol\";
import \"./Plain.sol\";
";
    let got: Vec<(String, Option<String>, String)> = solidity_imports(text)
        .into_iter()
        .map(|i| (i.bound, i.name, i.path))
        .collect();
    let own = |s: &str| s.to_owned();
    assert_eq!(
        got,
        [
            (
                own("ERC20"),
                Some(own("ERC20")),
                own("@openzeppelin/contracts/token/ERC20/ERC20.sol")
            ),
            (own("Base"), Some(own("ERC20")), own("./IERC20.sol")),
            (own("IERC20"), Some(own("IERC20")), own("./IERC20.sol")),
            (own("Ownable"), Some(own("Ownable")), own("./Ownable.sol")),
            (own("T"), None, own("./Token.sol")),
            (own("U"), None, own("./Util.sol")),
        ]
    );
}

#[test]
fn solidity_import_paths_resolve_as_the_compiler_does() {
    let root = std::env::temp_dir().join(format!("merl-sol-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let write = |p: &str, text: &str| {
        let p = root.join(p);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    };
    write(
        "remappings.txt",
        "@oz/=lib/openzeppelin-contracts/contracts/\n",
    );
    write(
        "foundry.toml",
        "[profile.default]\nremappings = [\n  \"ctx:@solmate/=lib/solmate/src/\",\n]\n",
    );
    write("lib/openzeppelin-contracts/contracts/token/ERC20.sol", "");
    write("lib/solmate/src/auth/Owned.sol", "");
    write("lib/forge-std/src/Test.sol", "");
    write(
        "node_modules/@openzeppelin/contracts/access/Ownable.sol",
        "",
    );
    write("contracts/shop/Token.sol", "");
    write("contracts/Fees.sol", "");
    let files: Vec<PathBuf> = [
        "lib/openzeppelin-contracts/contracts/token/ERC20.sol",
        "lib/solmate/src/auth/Owned.sol",
        "lib/forge-std/src/Test.sol",
        "contracts/shop/Token.sol",
        "contracts/Fees.sol",
    ]
    .map(PathBuf::from)
    .to_vec();
    let here = Path::new("contracts/shop/Token.sol");
    let file = |path: &str| solidity_file(&root, &files, here, path);
    let some = |p: &str| Some(PathBuf::from(p));
    assert_eq!(file("../Fees.sol"), some("contracts/Fees.sol"));
    assert_eq!(file("./Token.sol"), some("contracts/shop/Token.sol"));
    assert_eq!(
        file("@oz/token/ERC20.sol"),
        some("lib/openzeppelin-contracts/contracts/token/ERC20.sol")
    );
    assert_eq!(
        file("@solmate/auth/Owned.sol"),
        some("lib/solmate/src/auth/Owned.sol")
    );
    assert_eq!(
        file("@openzeppelin/contracts/access/Ownable.sol"),
        some("node_modules/@openzeppelin/contracts/access/Ownable.sol")
    );
    assert_eq!(
        file("forge-std/src/Test.sol"),
        some("lib/forge-std/src/Test.sol")
    );
    assert_eq!(file("contracts/Fees.sol"), some("contracts/Fees.sol"));
    assert_eq!(file("./Missing.sol"), None);
    assert_eq!(file("../../../x.sol"), None);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_path_of_a_solidity_import_is_followed() {
    let line = r#"import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";"#;
    let at = line.find("ERC20/ERC20").unwrap();
    assert_eq!(
        solidity_import_path(line, at).as_deref(),
        Some("@openzeppelin/contracts/token/ERC20/ERC20.sol")
    );
    assert_eq!(solidity_import_path(line, 9), None);
    assert_eq!(solidity_import_path(r#"string s = "./a.sol";"#, 13), None);
    assert_eq!(
        solidity_import_path(r#"import "./Token.sol" as T;"#, 10).as_deref(),
        Some("./Token.sol")
    );
}

#[test]
fn solidity_locals_are_parameters_returns_and_declarations_above() {
    let text = "\
contract Shop {
    uint256 fee;
    function weigh(uint256 grams, Order memory order) external returns (uint256 left) {
        uint256 fee = grams / 100;
        for (uint256 i = 0; i < 3; i++) {
            left += fee + i + order.grams;
        }
    }
    function settle(
        uint256 grams
    ) internal withinSupply(grams) {
        grams;
    }
    uint256 total = fee;
}
";
    let local = |line: usize, name: &str| -> Vec<usize> {
        bindings(Kind::Solidity, text, line, name)
            .iter()
            .map(|b| b.line)
            .collect()
    };
    assert_eq!(local(6, "left"), [3]);
    assert_eq!(local(6, "fee"), [4]);
    assert_eq!(local(6, "i"), [5]);
    assert_eq!(local(6, "order"), [3]);
    assert_eq!(local(4, "grams"), [3]);
    assert_eq!(local(4, "fee"), [4]);
    assert_eq!(local(12, "grams"), [10]);
    assert_eq!(local(11, "grams"), [10]);
    assert!(local(14, "total").is_empty());
    assert_eq!(local(3, "grams"), [3]);
    assert!(local(14, "fee").is_empty());
    assert!(local(6, "Shop").is_empty());
    let tuple = "\
contract Shop {
    function f() internal {
        (bool isAdmin, ) = check();
        (CallType callType, ExecType execType, , ) = mode.decode();
        assembly {
            let ptr := mload(0x40)
            let a, b := pair()
        }
        g(isAdmin, execType, ptr, b);
    }
}
";
    let local = |line: usize, name: &str| -> Vec<usize> {
        bindings(Kind::Solidity, tuple, line, name)
            .iter()
            .map(|b| b.line)
            .collect()
    };
    assert_eq!(local(9, "isAdmin"), [3]);
    assert_eq!(local(9, "execType"), [4]);
    assert_eq!(local(9, "callType"), [4]);
    assert!(local(9, "ptr").is_empty());
    assert_eq!(local(7, "ptr"), [6]);
    assert_eq!(local(7, "b"), [7]);
}

#[test]
fn solidity_is_told_by_its_extension() {
    assert_eq!(
        kind_of(Path::new("contracts/ShopToken.sol")),
        Some(Kind::Solidity)
    );
    assert_eq!(word_chars(Some(Kind::Solidity), false), "");
}
