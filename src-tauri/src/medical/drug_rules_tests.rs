use super::*;

const W: &str = r"[0-9A-Za-z_\u00C0-\u00D6\u00D8-\u00F6\u00F8-\u024F";

/// A rule in the exact shape the compiler emits, optionally excluded after titles.
fn pattern(core: &str, titles: &[&str]) -> String {
    let exclusions: String = titles
        .iter()
        .map(|t| format!("(?<!(?<!{W}]){t})"))
        .collect();
    format!(
        "{exclusions}(?<!{W}-])(?:(?<!['’])|(?<=(?<!{W}])[dlnjmtsc]['’])|(?<=qu['’])){core}(?!{W}-])"
    )
}

fn rules(list: &[(String, &str, &[&str])], always: &[u32]) -> DrugRules {
    // A rule without anchor words gets a variant that cannot be anchored.
    let jsonl: Vec<String> = list
        .iter()
        .enumerate()
        .map(|(i, (pattern, replacement, words))| {
            let variants: Vec<String> = if always.contains(&(i as u32)) {
                vec!["l'a.b".to_string()]
            } else {
                words.iter().map(|w| w.to_string()).collect()
            };
            serde_json::json!({"pattern": pattern, "replacement": replacement, "variants": variants})
                .to_string()
        })
        .collect();
    DrugRules::new(Arc::new(Bundled::from_jsonl(&[&jsonl.join("\n")])), &[])
}

fn bundled() -> DrugRules {
    DrugRules::new(Bundled::shared(), &[])
}

#[test]
fn replaces_on_word_boundaries_only() {
    let r = rules(
        &[(
            pattern("m[iy]rtazapine", &[]),
            "mirtazapine",
            &["myrtazapine"],
        )],
        &[],
    );
    assert_eq!(r.apply("sous myrtazapine."), "sous mirtazapine.");
    assert_eq!(r.apply("sous xmyrtazapine"), "sous xmyrtazapine");
    assert_eq!(r.apply("sous myrtazapines"), "sous myrtazapines");
    assert_eq!(r.apply("sous -myrtazapine"), "sous -myrtazapine");
    assert_eq!(r.apply("sous myrtazapine-x"), "sous myrtazapine-x");
}

#[test]
fn keeps_leading_capital_and_matches_case_insensitively() {
    let r = rules(
        &[(
            pattern("m[iy]rtazapine", &[]),
            "mirtazapine",
            &["myrtazapine"],
        )],
        &[],
    );
    assert_eq!(r.apply("Myrtazapine le soir"), "Mirtazapine le soir");
    assert_eq!(r.apply("MYRTAZAPINE"), "Mirtazapine");
}

#[test]
fn elided_articles_are_allowed_other_apostrophes_are_not() {
    let r = rules(&[(pattern("[eé]sope", &[]), "Ésope", &["esope"])], &[]);
    assert_eq!(r.apply("l'esope"), "l'Ésope");
    assert_eq!(r.apply("qu’esope"), "qu’Ésope");
    assert_eq!(r.apply("aujourd'esope"), "aujourd'esope");
    // "x" before the elided "l" is a word char: not an article.
    assert_eq!(r.apply("xl'esope"), "xl'esope");
}

#[test]
fn title_exclusions_block_only_right_after_the_title() {
    let titles: &[&str] = &["docteur ", r"dr\. "];
    let r = rules(&[(pattern("solian", titles), "Solian", &["solian"])], &[]);
    assert_eq!(
        r.apply("vu par le docteur solian"),
        "vu par le docteur solian"
    );
    assert_eq!(r.apply("vu par le Dr. solian"), "vu par le Dr. solian");
    assert_eq!(r.apply("sous solian"), "sous Solian");
    assert_eq!(r.apply("xdocteur solian"), "xdocteur Solian");
}

#[test]
fn later_rules_see_earlier_rewrites_but_earlier_ones_do_not_rerun() {
    let r = rules(
        &[
            (pattern("aa", &[]), "bb", &["aa"]),
            (pattern("bb", &[]), "cc", &["bb"]),
            (pattern("cc", &[]), "aa", &["cc"]),
        ],
        &[],
    );
    assert_eq!(r.apply("aa"), r.apply_naive("aa"));
    assert_eq!(r.apply("aa"), "aa");
    assert_eq!(r.apply("bb"), "aa");
}

#[test]
fn always_rules_run_without_anchor() {
    let r = rules(
        &[(
            pattern(r"l['’]ananas[\s-]+de[\s-]+l['’]umab", &[]),
            "lanadelumab",
            &[],
        )],
        &[0],
    );
    assert_eq!(r.apply("sous l'ananas de l'umab"), "sous lanadelumab");
}

#[test]
fn unsafe_or_malformed_rules_are_skipped() {
    let r = rules(
        &[
            (pattern("abc", &[]), "a\u{202e}b", &["abc"]),
            ("(?<=x)abc".to_string(), "def", &["abc"]),
            (pattern("abc", &[]), "ok", &["abc"]),
        ],
        &[],
    );
    assert_eq!(r.apply("abc"), "ok");
}

#[test]
fn fold_strips_accents_and_lowercases() {
    assert_eq!(fold("Médicament ÉTÉ"), "medicament ete");
}

fn mirtazapine_base() -> Arc<Bundled> {
    let rule = serde_json::json!({
        "pattern": pattern("m[iy]rtazapine", &["docteur "]),
        "replacement": "mirtazapine",
        "variants": ["myrtazapine"],
    });
    Arc::new(Bundled::from_jsonl(&[&rule.to_string()]))
}

fn edit(original: Option<String>, core: &str, replacement: &str, enabled: bool) -> RuleEdit {
    RuleEdit {
        original,
        core: core.to_string(),
        replacement: replacement.to_string(),
        enabled,
    }
}

#[test]
fn edits_disable_change_and_add_rules() {
    let base = mirtazapine_base();
    let original = Some(pattern("m[iy]rtazapine", &["docteur "]));

    let disabled = DrugRules::new(
        base.clone(),
        &[edit(original.clone(), "m[iy]rtazapine", "x", false)],
    );
    assert_eq!(disabled.apply("sous myrtazapine"), "sous myrtazapine");

    let renamed = DrugRules::new(
        base.clone(),
        &[edit(original.clone(), "m[iy]rtazapine", "Norset", true)],
    );
    assert_eq!(renamed.apply("sous myrtazapine"), "sous Norset");

    // A new core is no longer under the bundled anchor, yet it runs, and
    // keeps the bundled rule's title exclusions.
    let new_core = DrugRules::new(
        base.clone(),
        &[edit(original, "mirta", "mirtazapine", true)],
    );
    assert_eq!(new_core.apply("sous mirta"), "sous mirtazapine");
    assert_eq!(new_core.apply("docteur mirta"), "docteur mirta");

    // Added rules run before the bundled ones, on word boundaries.
    let added = DrugRules::new(base, &[edit(None, "dupont", "Dupond", true)]);
    assert_eq!(
        added.apply("Dupont sous myrtazapine"),
        "Dupond sous mirtazapine"
    );
    assert_eq!(added.apply("dupontel"), "dupontel");
    assert_eq!(added.apply("Dupont"), added.apply_naive("Dupont"));
}

#[test]
fn invalid_or_stale_edits_are_ignored() {
    let base = mirtazapine_base();
    let edits = [
        edit(
            Some(pattern("m[iy]rtazapine", &["docteur "])),
            "(",
            "x",
            true,
        ),
        edit(Some("no longer bundled".to_string()), "a", "b", true),
        edit(None, "(?<=a)b", "c", true),
    ];
    let rules = DrugRules::new(base, &edits);
    assert_eq!(rules.apply("sous myrtazapine ab"), "sous mirtazapine ab");
    assert!(validate_edit(&edits[0]).is_err());
    assert!(validate_edit(&edits[2]).is_err());
    assert!(validate_edit(&edit(None, "a", "\u{1b}[31m", true)).is_err());
}

#[test]
fn search_finds_rules_by_right_or_misheard_word() {
    let base = mirtazapine_base();
    assert_eq!(base.search("Mirtazapine", 10).len(), 1);
    assert_eq!(base.search("sous myrtazapine", 10).len(), 1);
    assert!(base.search("paracetamol", 10).is_empty());
    assert!(base.search("  ", 10).is_empty());
}

/// Outputs of ParakeetWeb's `applyDrugRules` (JavaScript) on real
/// transcriptions, with the bundled rules.
#[test]
fn bundled_rules_match_parakeetweb() {
    let r = bundled();
    let cases = [
        ("On atteint la dose maximale avec un comprimé de quatre mg de respéridone par jour.", "On atteint la dose maximale avec un comprimé de quatre mg de rispéridone par jour."),
        ("On réduit le selecoxyb à une gélule de cent mg par jour compte tenu de l'âge du patient.", "On réduit le célécoxib à une gélule de cent mg par jour compte tenu de l'âge du patient."),
        ("On réduit le Sélecoxybe à une gélule de cent mg par jour compte tenu de l'âge du patient.", "On réduit le Célécoxib à une gélule de cent mg par jour compte tenu de l'âge du patient."),
        ("Le patient est sous avapritini depuis six mois avec une bonne tolérance globale.", "Le patient est sous avapritinib depuis six mois avec une bonne tolérance globale."),
        ("Continuation de la phase de titration avec poneysimode incomprimé de sept mg, jour six.", "Continuation de la phase de titration avec ponésimod incomprimé de sept mg, jour six."),
        ("Surveillance de la calyémie recommandée sous endapamide incomprimé de un virgule cinq mg.", "Surveillance de la kaliémie recommandée sous indapamide incomprimé de un virgule cinq mg."),
        ("Examen réalisé pour une aménorrée primaire à l'aide d'un transducteur de 3,5 MHz.", "Examen réalisé pour une aménorrhée primaire à l'aide d'un transducteur de 3,5 MHz."),
        ("Surveillance de la calémie recommandée sous un dapamide incomprimé de un virgule cinq mg.", "Surveillance de la kaliémie recommandée sous indapamide incomprimé de un virgule cinq mg."),
        ("Bilan en autorino la ringologie.", "Bilan en otorhinolaryngologie."),
    ];
    for (input, expected) in cases {
        assert_eq!(r.apply(input), expected, "input: {input}");
    }
}

/// Every bundled rule has the expected shape and compiles. Slow (~35 s in
/// release), run with `cargo test --release -- --ignored`.
#[test]
#[ignore]
fn every_bundled_rule_compiles() {
    let r = bundled();
    let unusable: Vec<u32> = (0..r.len() as u32)
        .filter(|i| r.rule(*i).and_then(Rule::regex).is_none())
        .collect();
    assert!(unusable.is_empty(), "unusable rules: {unusable:?}");
}

/// Differential test against ParakeetWeb on a whole corpus: set
/// `MURMUREMED_DRUG_PARITY` to a JSON-lines file of `{"in": ..., "out": ...}`
/// produced by ParakeetWeb's `applyDrugRules`.
#[test]
#[ignore]
fn bundled_rules_match_parakeetweb_corpus() {
    let Ok(path) = std::env::var("MURMUREMED_DRUG_PARITY") else {
        return;
    };
    let r = bundled();
    let mut diffs = 0;
    let mut total = 0;
    for line in std::fs::read_to_string(path).unwrap().lines() {
        let case: serde_json::Value = serde_json::from_str(line).unwrap();
        let (input, expected) = (case["in"].as_str().unwrap(), case["out"].as_str().unwrap());
        total += 1;
        let got = r.apply(input);
        if got != expected {
            diffs += 1;
            eprintln!("IN:   {input}\nJS:   {expected}\nRUST: {got}\n");
        }
    }
    assert_eq!(diffs, 0, "{diffs}/{total} differ");
}
