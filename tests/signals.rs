use oli_demo_rust_app::model::*;
use oli_demo_rust_app::signals::*;

fn bars(closes: &[f64]) -> Vec<Bar> {
    closes
        .iter()
        .enumerate()
        .map(|(i, c)| Bar {
            t: i as i64 * 86_400,
            close: *c,
            volume: 1_000_000.0,
        })
        .collect()
}

fn find<'a>(sigs: &'a [Signal], name: &str) -> &'a Signal {
    sigs.iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("no signal {name}"))
}

fn company_with(f: Fundamentals) -> Company {
    Company {
        ticker: "T".into(),
        name: "Test".into(),
        fundamentals: f,
        ..Default::default()
    }
}

#[test]
fn pe_rules() {
    let c = build_card(
        &company_with(Fundamentals {
            pe: Some(20.0),
            ..Default::default()
        }),
        0,
    );
    let s = find(&c.fundamentals, "P/E Ratio");
    assert_eq!((s.label.as_str(), s.strength), ("Fair value", 70));

    let c = build_card(&company_with(Fundamentals::default()), 0);
    let s = find(&c.fundamentals, "P/E Ratio");
    assert_eq!(s.label, "N/A");
    assert!(!s.available);

    let c = build_card(
        &company_with(Fundamentals {
            pe: Some(-5.0),
            ..Default::default()
        }),
        0,
    );
    let s = find(&c.fundamentals, "P/E Ratio");
    assert_eq!((s.label.as_str(), s.strength), ("Unprofitable", 20));
}

#[test]
fn other_fundamentals_rules() {
    let f = Fundamentals {
        roe_pct: Some(20.0),
        debt_to_equity: Some(0.3),
        pb: Some(2.0),
        eps_growth_pct: Some(25.0),
        ..Default::default()
    };
    let c = build_card(&company_with(f), 0);
    let roe = find(&c.fundamentals, "ROE");
    assert_eq!((roe.label.as_str(), roe.strength), ("Good", 50));
    let de = find(&c.fundamentals, "Debt/Equity");
    assert_eq!((de.label.as_str(), de.strength), ("Low", 88));
    let pb = find(&c.fundamentals, "P/B Ratio");
    assert_eq!((pb.label.as_str(), pb.strength), ("Fair", 70));
    assert_eq!(
        find(&c.fundamentals, "EPS Growth").label,
        "Strong EPS growth"
    );
}

#[test]
fn analyst_consensus_label() {
    let mut co = company_with(Fundamentals::default());
    co.analysts = Some(Analysts {
        strong_buy: 10,
        buy: 20,
        hold: 9,
        sell: 0,
        strong_sell: 0,
    });
    let c = build_card(&co, 0);
    assert_eq!(
        find(&c.momentum, "Analyst Sentiment").label,
        "80% buy consensus (39 analysts)"
    );
}

#[test]
fn earnings_surprise_label() {
    let mut co = company_with(Fundamentals::default());
    co.earnings = vec![Earning {
        actual: 1.1,
        estimate: 1.0,
    }];
    let c = build_card(&co, 0);
    let s = find(&c.momentum, "Earnings Surprise");
    assert_eq!(
        (s.label.as_str(), s.strength),
        ("+10.0% avg EPS surprise", 70)
    );
}

#[test]
fn price_vs_sma50_label() {
    let mut co = company_with(Fundamentals::default());
    co.bars = bars(&[100.0; 50]);
    co.price = Some(102.0);
    let c = build_card(&co, 0);
    assert_eq!(
        find(&c.momentum, "Price vs 50d SMA").label,
        "+2.0% vs 50d SMA"
    );
}

#[test]
fn headline_sentiment_direction() {
    assert!(headline_sentiment("Disney beats estimates as shares surge") > 0.0);
    assert!(headline_sentiment("Shares plunge after lawsuit") < 0.0);
    assert_eq!(headline_sentiment("Disney to hold annual meeting"), 0.0);
}

#[test]
fn full_card_shape() {
    let closes: Vec<f64> = (0..260)
        .map(|i| 100.0 + (i as f64 * 0.3).sin() * 5.0 + i as f64 * 0.05)
        .collect();
    let mut co = company_with(Fundamentals::default());
    co.bars = bars(&closes);
    let c = build_card(&co, 42);
    assert_eq!(c.as_of, 42);
    assert_eq!(c.spark.len(), 14);
    assert_eq!(c.spark.last().copied(), closes.last().copied());
    let names: Vec<&str> = c.momentum.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "RSI (14)",
            "MACD",
            "Price vs 50d SMA",
            "Bollinger Band",
            "Volume",
            "Analyst Sentiment",
            "Earnings Surprise",
            "News Sentiment"
        ]
    );
    let names: Vec<&str> = c.fundamentals.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "P/E Ratio",
            "ROE",
            "Debt/Equity",
            "EPS Growth",
            "Revenue Growth",
            "P/B Ratio",
            "Price vs 200d SMA"
        ]
    );
    assert!(find(&c.momentum, "RSI (14)").available);
    assert!(find(&c.momentum, "Volume").available);
    assert!(find(&c.fundamentals, "Price vs 200d SMA").available);
}

#[test]
fn empty_company_is_all_na_without_panicking() {
    let c = build_card(&company_with(Fundamentals::default()), 0);
    assert!(c.spark.is_empty());
    assert!(c.change_pct.is_none());
    assert!(c
        .momentum
        .iter()
        .all(|s| !s.available && s.label == "N/A" && s.strength == 50));
}
