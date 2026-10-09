use super::*;

fn shaped(line: &str, word: &str) -> bool {
    Regex::new(&haskell_patterns(word).join("|"))
        .unwrap()
        .is_match(line)
}

fn declares(text: &str, line: usize, word: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    shaped(lines[line - 1], word) && haskell_declares(&lines, line, word)
}

#[test]
fn haskell_declaration_forms() {
    for (line, word) in [
        ("formatPrice :: Money -> String", "formatPrice"),
        ("parseA, parseB :: Parser Int", "parseB"),
        ("go' :: Int", "go'"),
        ("data Shape = Circle Double | Square Double", "Shape"),
        ("data Shape = Circle Double | Square Double", "Circle"),
        ("data Shape = Circle Double | Square Double", "Square"),
        ("newtype Money = Money { cents :: Int }", "Money"),
        ("newtype Money = Money { cents :: Int }", "cents"),
        ("type Name = Text", "Name"),
        ("type family F a", "F"),
        ("data family D a", "D"),
        ("pattern Zero = Money 0", "Zero"),
        ("pattern Head x <- (x:_)", "Head"),
        ("class (Eq a) => Pretty a where", "Pretty"),
        ("class Monad m => MonadLog m where", "MonadLog"),
        ("  | Flat Money", "Flat"),
        ("  = Percent Int", "Percent"),
        ("  , perKilo :: Int", "perKilo"),
        ("  { tariffName :: String", "tariffName"),
        ("  Lit :: Int -> Expr Int", "Lit"),
        ("  pretty :: a -> String", "pretty"),
        ("weighBand 0 = \"none\"", "weighBand"),
        ("weighBand n", "weighBand"),
        ("label m = go (formatPrice m)", "label"),
        ("f (Money c) xs@(x:_) = c", "f"),
        ("f Foo{..} = 1", "f"),
    ] {
        assert!(shaped(line, word), "{line}: {word}");
    }
}

#[test]
fn haskell_refusals() {
    for (line, word) in [
        ("instance Show Money where", "Show"),
        ("instance Show Money where", "Money"),
        ("  deriving (Show, Eq)", "Show"),
        ("formatPrice :: Money -> String", "Money"),
        ("  case s of Circle r -> r", "Circle"),
        ("reprice t = t { perKilo = 0 }", "perKilo"),
        ("m <> n = m", "m"),
        ("x `plus` y = x", "x"),
        ("type instance F Int = Bool", "F"),
        ("pattern Zero :: Money", "Zero"),
        ("formatPriceX = 1", "formatPrice"),
        ("go = foldl' f", "foldl"),
    ] {
        assert!(!shaped(line, word), "{line}: {word}");
    }
}

#[test]
fn haskell_declares_where_the_name_is_seen() {
    let text = "\
weigh :: Int -> Bool
weigh k = go k
  where
    go n = n > 0
    | n < 0 = False

total xs = sum xs
total' = 0

band 0 = 1
band n
  | n > 0 = 2
band _ = 3

class Pretty a where
  pretty :: a -> String

instance Pretty Int where
  pretty :: Int -> String
  pretty = show

data Coupon
  = Percent Int
  | Flat Int
";
    assert!(declares(text, 1, "weigh"));
    assert!(
        !declares(text, 2, "weigh"),
        "an equation under its signature"
    );
    assert!(!declares(text, 4, "go"), "a local");
    assert!(declares(text, 7, "total"), "no signature: the equation");
    assert!(declares(text, 8, "total'"));
    assert!(declares(text, 10, "band"), "the first of a run");
    assert!(!declares(text, 11, "band"));
    assert!(!declares(text, 13, "band"));
    assert!(declares(text, 16, "pretty"), "a method of the class");
    assert!(!declares(text, 19, "pretty"), "an instance's");
    assert!(declares(text, 23, "Percent"));
    assert!(declares(text, 24, "Flat"));
}

fn local(text: &str, line: usize, name: &str) -> Vec<usize> {
    bindings(Kind::Haskell, text, line, name)
        .iter()
        .map(|b| b.line1)
        .collect()
}

#[test]
fn haskell_locals_stay_in_their_declaration() {
    let text = "\
label :: Money -> String
label m = go (formatPrice m)
  where
    go :: String -> String
    go s = s

lookup n = go
  where
    go [] = Nothing
    go (x : xs)
      | x == n = Just x
      | otherwise = go xs

ship c = do
  let limit = 3
  ok <- pure c
  print (\\k -> k + limit, ok)
  case c of
    Just v -> v
";
    assert_eq!(local(text, 2, "go"), [4], "the signature of a local");
    assert_eq!(local(text, 2, "m"), [2], "a parameter");
    assert_eq!(local(text, 5, "s"), [5]);
    assert_eq!(
        local(text, 7, "go"),
        [9],
        "this declaration's go, the first equation"
    );
    assert_eq!(local(text, 12, "go"), [9]);
    assert_eq!(local(text, 11, "x"), [10]);
    assert_eq!(local(text, 11, "n"), [7]);
    assert_eq!(local(text, 17, "limit"), [15]);
    assert_eq!(local(text, 17, "ok"), [16]);
    assert_eq!(
        local(text, 17, "k"),
        [17],
        "a lambda's parameter on its line"
    );
    assert_eq!(local(text, 19, "v"), [19], "a case alternative's");
    assert!(local(text, 2, "label").is_empty());
    assert!(
        local(text, 19, "Just").is_empty(),
        "a constructor is no local"
    );
    assert!(local(text, 14, "go").is_empty(), "another declaration's go");
}

#[test]
fn haskell_methods_are_no_locals() {
    let text = "\
class Pretty a where
  pretty :: a -> String
  pretty x = show x

instance Pretty Int where
  pretty n = show n
";
    assert!(local(text, 3, "pretty").is_empty());
    assert!(local(text, 6, "pretty").is_empty());
    assert_eq!(local(text, 6, "n"), [6]);
}

#[test]
fn haskell_imports_bind_names_and_qualifiers() {
    let text = "\
import Shop.Money (Money (..), formatPrice) -- prices
import qualified Data.Map as Map
import Data.List hiding (sortOn)
import Shop.Courier
  ( Courier (Courier, maxKilos)
  , weigh
  )
import Data.Text qualified as T
";
    let imports = haskell_imports(text);
    let rows: Vec<_> = (imports.iter())
        .map(|i| (i.module.as_str(), i.alias.as_deref(), i.names.clone()))
        .collect();
    assert_eq!(
        rows,
        [
            (
                "Shop.Money",
                None,
                vec!["Money".to_owned(), "formatPrice".into()]
            ),
            ("Data.Map", Some("Map"), vec![]),
            ("Data.List", None, vec![]),
            (
                "Shop.Courier",
                None,
                vec![
                    "Courier".to_owned(),
                    "Courier".into(),
                    "maxKilos".into(),
                    "weigh".into()
                ]
            ),
            ("Data.Text", Some("T"), vec![]),
        ]
    );
    let line = "import qualified Shop.Money as M";
    assert_eq!(
        haskell_import_module(line, 20).as_deref(),
        Some("Shop.Money")
    );
    assert_eq!(haskell_import_module(line, 3), None);
    let files = [
        PathBuf::from("src/Shop/Money.hs"),
        PathBuf::from("Money.hs"),
    ];
    assert_eq!(
        haskell_files("Shop.Money", &files),
        [PathBuf::from("src/Shop/Money.hs")]
    );
}

#[test]
fn haskell_literals_hide_declarations() {
    let text = "\
{- data A = A
   {- nested -}
data B = B
-}
data C = C -- data D = D
query = [sql|
data E = E
|]
data F = F
s = \"data G\" ++ [c | c <- \"x\"]
x --> y = 1
data H = H
c = '\"'
data I = I
";
    let literal = literal_lines(Kind::Haskell, text);
    let hidden: Vec<usize> = (literal.iter().enumerate())
        .filter(|(_, l)| **l)
        .map(|(i, _)| i + 1)
        .collect();
    assert_eq!(hidden, [2, 3, 4, 7, 8]);
}

#[test]
fn haskell_symbols_are_top_level_declarations() {
    let hs = |line| {
        let names: Vec<String> = (listed(Kind::Haskell, line).into_iter())
            .filter(|n| !HASKELL_RESERVED.contains(&n.as_str()))
            .collect();
        assert!(names.len() <= 1, "{line}: {names:?}");
        names.into_iter().next()
    };
    for (line, name) in [
        ("formatPrice :: Money -> String", "formatPrice"),
        ("data Shape = Circle Double", "Shape"),
        ("newtype Money = Money Int", "Money"),
        ("type Name = Text", "Name"),
        ("type family F a", "F"),
        ("class (Eq a) => Pretty a where", "Pretty"),
        ("pattern Zero = Money 0", "Zero"),
        ("total xs = sum xs", "total"),
    ] {
        assert_eq!(hs(line).as_deref(), Some(name), "{line}");
    }
    for line in [
        "  go s = s",
        "  | Flat Money",
        "  , cents :: Int",
        "  deriving (Show)",
        "m <> n = m",
    ] {
        assert_eq!(hs(line), None, "{line}");
    }
    let text = "total xs = sum xs\ntotal [] = 0\nweigh :: Int\nweigh = 1\nimport Data.List\ninstance Show Money where\nmakeLenses ''Tariff\n{-\nghost = 1\n-}\n";
    let lines: Vec<&str> = text.lines().collect();
    let code = HaskellCode::new(&lines);
    let kept: Vec<usize> = [
        (1, "total"),
        (2, "total"),
        (3, "weigh"),
        (4, "weigh"),
        (5, "import"),
        (6, "instance"),
        (7, "makeLenses"),
        (9, "ghost"),
    ]
    .into_iter()
    .filter(|&(l, n)| haskell_symbol(&code, l, n))
    .map(|(l, _)| l)
    .collect();
    assert_eq!(kept, [1, 3]);
    assert!(!shared_symbols(Some(Kind::Haskell)));
}

#[test]
fn a_haskell_name_holds_its_primes() {
    let line = "  x = foldl' f go' 'a' don't";
    let word = |col| definition_word(Some(Kind::Haskell), line, col).map(|(_, w)| w);
    assert_eq!(word(7), Some("foldl'"));
    assert_eq!(word(16), Some("go'"));
    assert_eq!(word(20), Some("a"));
    assert_eq!(word(25), Some("don't"));
    assert_eq!(kind_of(Path::new("src/Shop/Money.hs")), Some(Kind::Haskell));
    assert_eq!(kind_of(Path::new("Setup.lhs")), None);
}

#[test]
fn haskell_declares_only_in_code() {
    let text = "\
{-
foo :: Int -> Int
-}
foo x = x + 1

#if FAST
scale factor = factor * 2
#else
scale factor = factor * 3
#endif

data Coupon
  = Percent Int -- ^ off, a => b
  | Flat Int

main = print (0 :: Int, total :: Int -> Int)

total' = 0
total xs = sum xs

bar
  :: Int
bar = 1

makeLenses ''Coupon
";
    assert!(declares(text, 4, "foo"), "a signature in a comment is none");
    assert!(declares(text, 7, "scale"));
    assert!(!declares(text, 9, "scale"), "CPP splits no run");
    assert!(
        declares(text, 13, "Percent"),
        "a comment after a constructor"
    );
    assert!(
        !declares(text, 16, "total"),
        "an annotation in an expression"
    );
    assert!(declares(text, 19, "total"), "total' is another name");
    assert!(declares(text, 21, "bar"), "a signature over two lines");
    assert!(!declares(text, 23, "bar"));
    assert!(
        !declares(text, 25, "makeLenses"),
        "a Template Haskell splice"
    );
    assert_eq!(
        local(text, 7, "factor"),
        [7],
        "CPP is no declaration of its own"
    );
}

#[test]
fn haskell_locals_in_corners() {
    let text = "\
label m = go m
  where
    go :: Int
    go s = go s

total = go 0 + x
  where
    go x = x

instance P Int where
  p n = go n
    where go k = k

f = do
  print ok
  ok <- pure 1

run \"build\" = build
";
    assert_eq!(
        local(text, 4, "go"),
        [3],
        "a local's signature from its own body"
    );
    assert!(local(text, 6, "x").is_empty(), "a sibling's parameter");
    assert_eq!(local(text, 11, "go"), [12], "a local under a method");
    assert!(local(text, 15, "ok").is_empty(), "bound below its use");
    assert!(
        local(text, 18, "build").is_empty(),
        "a string is no parameter"
    );
}

#[test]
fn haskell_imports_skip_comments_and_know_qualified() {
    let text = "\
{-
import Data.Text (render)
-}
import Shop.Courier
  ( weigh -- scale
  , ship
  )
import qualified Data.Map.Strict as M (Map, insert)
";
    let imports = haskell_imports(text);
    let rows: Vec<_> = (imports.iter())
        .map(|i| (i.module.as_str(), i.qualified, i.names.clone()))
        .collect();
    assert_eq!(
        rows,
        [
            (
                "Shop.Courier",
                false,
                vec!["weigh".to_owned(), "ship".into()]
            ),
            (
                "Data.Map.Strict",
                true,
                vec!["Map".to_owned(), "insert".into()]
            ),
        ]
    );
}

#[test]
fn a_haskell_quote_is_no_part_of_the_name() {
    assert_eq!(haskell_whole("makeLenses ''Config", "Config"), Some(13));
    assert_eq!(haskell_whole("x = 'Just", "Just"), Some(5));
    assert_eq!(haskell_whole("don't = 1", "t"), None);
    assert_eq!(haskell_whole("foldl' f", "foldl"), None);
}
