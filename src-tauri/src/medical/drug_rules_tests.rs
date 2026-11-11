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
    let rules: Vec<(String, String)> = list
        .iter()
        .map(|(p, r, _)| (p.clone(), r.to_string()))
        .collect();
    let mut anchors: HashMap<String, Vec<u32>> = HashMap::new();
    for (i, (_, _, words)) in list.iter().enumerate() {
        for w in *words {
            anchors.entry(w.to_string()).or_default().push(i as u32);
        }
    }
    let json =
        serde_json::json!({"format": 1, "rules": rules, "anchors": anchors, "always": always});
    DrugRules::from_json(&json.to_string()).unwrap()
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

/// Outputs of ParakeetWeb's `applyDrugRules` (JavaScript) on real
/// transcriptions, with the bundled rules.
#[test]
fn bundled_rules_match_parakeetweb() {
    let r = DrugRules::bundled().expect("bundled rules load");
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
    let r = DrugRules::bundled().expect("bundled rules load");
    let unusable: Vec<usize> = (0..r.len()).filter(|i| r.regex(*i).is_none()).collect();
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
    let r = DrugRules::bundled().expect("bundled rules load");
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
