use super::types::{AdvisorDomain, AlertKind, CityContext};

// Templates loaded at compile time from assets/prompts/.
const TMPL_CITY_NAME: &str = include_str!("../../assets/prompts/city_name.txt");
const TMPL_NEWSPAPER: &str = include_str!("../../assets/prompts/newspaper.txt");
const TMPL_ADVISOR: &str = include_str!("../../assets/prompts/advisor.txt");
const TMPL_ALERT: &str = include_str!("../../assets/prompts/alert.txt");
const TMPL_NEWSPAPER_ARTICLE: &str = include_str!("../../assets/prompts/newspaper_article.txt");
const RULES_SUMMARY: &str = include_str!("../../assets/prompts/rules_summary.txt");

/// Replace all `{{key}}` occurrences in `template` with the corresponding value.
fn render(template: &str, vars: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for &(key, value) in vars {
        let placeholder = format!("{{{{{key}}}}}");
        out = out.replace(&placeholder, value);
    }
    out
}

fn month_abbrev(month: u8) -> &'static str {
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "???",
    }
}

fn month_name(month: u8) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    }
}

fn format_context(ctx: &CityContext) -> String {
    let power_status = if ctx.power_consumed_mw > ctx.power_produced_mw {
        "SHORTAGE"
    } else {
        "OK"
    };
    let water_status = if ctx.water_consumed > ctx.water_produced {
        "SHORTAGE"
    } else {
        "OK"
    };

    format!(
        "City: {name}, Date: {month} {year}\n\
         Population: {pop} (change: {delta:+})\n\
         Treasury: ${treasury}, Monthly income: ${income}\n\
         Tax rates: Res {tr}%, Comm {tc}%, Ind {ti}%\n\
         Demand: Res {dr:.2}, Comm {dc:.2}, Ind {di:.2}\n\
         Power: {pp} MW produced / {pc} MW consumed ({ps})\n\
         Water: {wp} produced / {wc} consumed ({ws})\n\
         Avg pollution: {pol}/255, Avg crime: {cri}/255\n\
         Avg land value: {lv}/255, Avg fire risk: {fr}/255\n\
         Active fires: {fires}, Trip success: {trips:.0}%\n\
         Services: {schools} schools, {hosp} hospitals, {police} police, {fire} fire stations, {parks} parks",
        name = ctx.city_name,
        month = month_abbrev(ctx.month),
        year = ctx.year,
        pop = ctx.population,
        delta = ctx.pop_delta,
        treasury = ctx.treasury,
        income = ctx.last_income,
        tr = ctx.tax_res,
        tc = ctx.tax_comm,
        ti = ctx.tax_ind,
        dr = ctx.demand_res,
        dc = ctx.demand_comm,
        di = ctx.demand_ind,
        pp = ctx.power_produced_mw,
        pc = ctx.power_consumed_mw,
        ps = power_status,
        wp = ctx.water_produced,
        wc = ctx.water_consumed,
        ws = water_status,
        pol = ctx.avg_pollution,
        cri = ctx.avg_crime,
        lv = ctx.avg_land_value,
        fr = ctx.avg_fire_risk,
        fires = ctx.active_fires,
        trips = ctx.trip_success_rate * 100.0,
        schools = ctx.num_schools,
        hosp = ctx.num_hospitals,
        police = ctx.num_police,
        fire = ctx.num_fire_stations,
        parks = ctx.num_parks,
    )
}

fn strongest_demand(ctx: &CityContext) -> (&'static str, f32) {
    [
        ("residential", ctx.demand_res),
        ("commercial", ctx.demand_comm),
        ("industrial", ctx.demand_ind),
    ]
    .into_iter()
    .max_by(|a, b| a.1.total_cmp(&b.1))
    .unwrap_or(("residential", ctx.demand_res))
}

fn classify_city_lead(ctx: &CityContext) -> &'static str {
    if ctx.active_fires > 0 {
        "multiple fires are demanding an emergency response"
    } else if ctx.power_consumed_mw > ctx.power_produced_mw {
        "the power grid is falling short of demand"
    } else if ctx.water_consumed > ctx.water_produced {
        "water service is under strain"
    } else if ctx.treasury < 0 || ctx.last_income < 0 {
        "City Hall is wrestling with a budget problem"
    } else if ctx.trip_success_rate < 0.60 {
        "the commute is breaking down across town"
    } else if ctx.avg_crime >= 140 {
        "crime has become a major civic concern"
    } else if ctx.avg_pollution >= 140 {
        "pollution is hanging over the city"
    } else if ctx.pop_delta > 0 {
        "the city is growing and trying to keep up"
    } else {
        "the city is holding steady with mixed expectations"
    }
}

fn format_newspaper_briefing(ctx: &CityContext) -> String {
    let power_line = if ctx.power_consumed_mw > ctx.power_produced_mw {
        format!(
            "power supply falls short of demand by {} MW",
            ctx.power_consumed_mw - ctx.power_produced_mw
        )
    } else {
        format!(
            "power supply is ahead of demand by {} MW",
            ctx.power_produced_mw - ctx.power_consumed_mw
        )
    };

    let water_line = if ctx.water_consumed > ctx.water_produced {
        format!(
            "water demand exceeds supply by {} units",
            ctx.water_consumed - ctx.water_produced
        )
    } else {
        format!(
            "water supply is ahead by {} units",
            ctx.water_produced - ctx.water_consumed
        )
    };

    let (demand_kind, demand_value) = strongest_demand(ctx);

    format!(
        "Lead issue: {lead}\n\
         Growth: population {population} ({delta:+} this month)\n\
         Budget: treasury ${treasury}; monthly income ${income}\n\
         Utilities: {power}; {water}\n\
         Transit: trip success is {trips:.0}%\n\
         Environment: pollution {pollution}/255, crime {crime}/255, fire risk {fire}/255, land value {land}/255\n\
         Demand: strongest {demand_kind} demand at {demand_value:.2}\n\
         Services: {schools} schools, {hospitals} hospitals, {police} police stations, {fire_stations} fire stations, {parks} parks",
        lead = classify_city_lead(ctx),
        population = ctx.population,
        delta = ctx.pop_delta,
        treasury = ctx.treasury,
        income = ctx.last_income,
        power = power_line,
        water = water_line,
        trips = ctx.trip_success_rate * 100.0,
        pollution = ctx.avg_pollution,
        crime = ctx.avg_crime,
        fire = ctx.avg_fire_risk,
        land = ctx.avg_land_value,
        demand_kind = demand_kind,
        demand_value = demand_value,
        schools = ctx.num_schools,
        hospitals = ctx.num_hospitals,
        police = ctx.num_police,
        fire_stations = ctx.num_fire_stations,
        parks = ctx.num_parks,
    )
}

fn format_newspaper_color(ctx: &CityContext) -> String {
    let civic_brag = if ctx.pop_delta > 0 {
        format!(
            "moving vans keep arriving and citizens can feel the city stretching into its next chapter"
        )
    } else if ctx.treasury >= 0 && ctx.last_income >= 0 {
        "the books look calm enough for City Hall to brag at dinner parties".to_string()
    } else if ctx.trip_success_rate >= 0.75 {
        "commuters still grumble, but most of them are getting where they meant to go".to_string()
    } else {
        "people still believe the city can get its act together if someone finally fixes the obvious problem"
            .to_string()
    };

    let resident_gripe = classify_city_lead(ctx).to_string();
    let sponsor_pitch = if ctx.power_consumed_mw > ctx.power_produced_mw {
        "sell generators, extension cords, and brave promises about the power grid".to_string()
    } else if ctx.water_consumed > ctx.water_produced {
        "promote water tanks, neat lawns, and suspiciously optimistic plumbing".to_string()
    } else if ctx.demand_res > ctx.demand_comm && ctx.demand_res > ctx.demand_ind {
        "sell furniture, moving boxes, and dreams of the perfect starter neighborhood".to_string()
    } else if ctx.demand_comm > ctx.demand_ind {
        "sell storefront signs, coffee, and the idea that every avenue needs one more shop"
            .to_string()
    } else {
        "sell work boots, used machinery, and confidence about the city's productive future"
            .to_string()
    };
    let street_mood = if ctx.avg_crime >= 140 {
        "residents sound alert, sarcastic, and eager for somebody competent to take charge"
            .to_string()
    } else if ctx.avg_pollution >= 140 {
        "residents sound proud of the skyline and slightly offended by the air quality".to_string()
    } else {
        "residents sound like they have just enough hope to keep gossiping instead of packing"
            .to_string()
    };

    format!(
        "Civic brag: {civic_brag}\n\
         Resident gripe: {resident_gripe}\n\
         Sponsor angle: {sponsor_pitch}\n\
         Street mood: {street_mood}"
    )
}

pub fn city_name_prompt() -> String {
    TMPL_CITY_NAME.to_string()
}

pub fn newspaper_article_prompt(ctx: &CityContext) -> String {
    let month_name = month_name(ctx.month);
    let year = ctx.year.to_string();
    let city_status = format_context(ctx);
    let city_briefing = format_newspaper_briefing(ctx);
    let city_color = format_newspaper_color(ctx);
    render(
        TMPL_NEWSPAPER_ARTICLE,
        &[
            ("city_name", &ctx.city_name),
            ("month_name", month_name),
            ("year", &year),
            ("city_status", &city_status),
            ("city_briefing", &city_briefing),
            ("city_color", &city_color),
        ],
    )
}

pub fn newspaper_prompt(ctx: &CityContext) -> String {
    let year = ctx.year.to_string();
    let population = ctx.population.to_string();
    let treasury = ctx.treasury.to_string();
    let city_status = format_context(ctx);
    let city_briefing = format_newspaper_briefing(ctx);
    render(
        TMPL_NEWSPAPER,
        &[
            ("city_name", &ctx.city_name),
            ("population", &population),
            ("treasury", &treasury),
            ("city_status", &city_status),
            ("month_name", month_name(ctx.month)),
            ("year", &year),
            ("city_briefing", &city_briefing),
        ],
    )
}

pub fn advisor_prompt(ctx: &CityContext, domain: &AdvisorDomain) -> String {
    let role = match domain {
        AdvisorDomain::Economy => "Finance Advisor",
        AdvisorDomain::CityPlanning => "City Planning Advisor",
        AdvisorDomain::Education => "Education Advisor",
        AdvisorDomain::Safety => "Safety Advisor",
        AdvisorDomain::Transport => "Transport Advisor",
    };

    render(
        TMPL_ADVISOR,
        &[
            ("advisor_role", role),
            ("city_status", &format_context(ctx)),
        ],
    )
}

pub fn alert_prompt(ctx: &CityContext, kind: &AlertKind) -> String {
    let situation = match kind {
        AlertKind::Fire { count } => {
            format!("{count} tiles are currently on fire in the city")
        }
        AlertKind::Deficit { treasury } => {
            format!("the city treasury is at ${treasury} and running a deficit")
        }
        AlertKind::Brownout => {
            "the power grid cannot meet demand and brownouts are occurring".to_string()
        }
        AlertKind::WaterShortage => {
            "water supply is insufficient and parts of the city are going dry".to_string()
        }
        AlertKind::TrafficCrisis => format!(
            "only {:.0}% of trips are succeeding — the transport network is failing",
            ctx.trip_success_rate * 100.0
        ),
        AlertKind::Tornado => {
            "a tornado has struck the city, leaving destruction in its path".to_string()
        }
        AlertKind::Flood => "flooding has hit areas near the waterfront".to_string(),
        AlertKind::PlantExpiring {
            plant_type,
            months_left,
        } => {
            format!("a {plant_type} power plant has only {months_left} months of life remaining")
        }
    };

    let pop = ctx.population.to_string();
    let year = ctx.year.to_string();

    render(
        TMPL_ALERT,
        &[
            ("city_name", &ctx.city_name),
            ("population", &pop),
            ("year", &year),
            ("situation", &situation),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::textgen::types::sample_context;

    #[test]
    fn render_replaces_variables() {
        let result = render(
            "Hello {{name}}, welcome to {{place}}!",
            &[("name", "Mayor"), ("place", "Springfield")],
        );
        assert_eq!(result, "Hello Mayor, welcome to Springfield!");
    }

    #[test]
    fn render_replaces_repeated_variable() {
        let result = render("{{x}} and {{x}}", &[("x", "A")]);
        assert_eq!(result, "A and A");
    }

    #[test]
    fn render_leaves_unknown_placeholders() {
        let result = render("Hello {{name}}", &[]);
        assert_eq!(result, "Hello {{name}}");
    }

    #[test]
    fn templates_are_loaded() {
        assert!(!TMPL_CITY_NAME.is_empty());
        assert!(!TMPL_NEWSPAPER.is_empty());
        assert!(!TMPL_ADVISOR.is_empty());
        assert!(!TMPL_ALERT.is_empty());
        assert!(!RULES_SUMMARY.is_empty());
    }

    #[test]
    fn rules_summary_contains_game_content() {
        assert!(RULES_SUMMARY.contains("Game rules summary"));
        assert!(RULES_SUMMARY.contains("power") || RULES_SUMMARY.contains("Power"));
    }

    #[test]
    fn city_name_prompt_is_short() {
        let prompt = city_name_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.len() < 1000);
    }

    #[test]
    fn newspaper_prompt_includes_city_name_and_context() {
        let ctx = sample_context();
        let prompt = newspaper_prompt(&ctx);
        assert!(prompt.contains("Testville"));
        assert!(prompt.contains("5000"));
        assert!(prompt.contains("Lead issue:"));
    }

    #[test]
    fn newspaper_prompt_includes_tribune() {
        let ctx = sample_context();
        let prompt = newspaper_prompt(&ctx);
        assert!(prompt.contains("Tribune"));
    }

    #[test]
    fn newspaper_prompt_has_no_unreplaced_placeholders() {
        let ctx = sample_context();
        let prompt = newspaper_prompt(&ctx);
        assert!(
            !prompt.contains("{{"),
            "unreplaced placeholder in newspaper prompt"
        );
    }

    #[test]
    fn newspaper_article_prompt_includes_briefing_and_structure() {
        let ctx = sample_context();
        let prompt = newspaper_article_prompt(&ctx);
        assert!(prompt.contains("Lead issue:"));
        assert!(prompt.contains("PAGE 1: FRONT PAGE"));
        assert!(prompt.contains("SECTION: LEAD STORY"));
        assert!(prompt.contains("SECTION: LETTERS FROM READERS"));
        assert!(prompt.contains("SECTION: CLASSIFIEDS"));
        assert!(prompt.contains("SECTION: WEATHER DESK"));
        assert!(prompt.contains("Dear Mayor,"));
        assert!(prompt.contains("Entertainment hooks:"));
        assert!(!prompt.contains("{{"));
    }

    #[test]
    fn advisor_prompt_includes_domain_role() {
        let ctx = sample_context();
        let prompt = advisor_prompt(&ctx, &AdvisorDomain::Economy);
        assert!(prompt.contains("Finance Advisor"));
        assert!(prompt.contains("Testville"));

        let prompt = advisor_prompt(&ctx, &AdvisorDomain::Safety);
        assert!(prompt.contains("Safety Advisor"));
    }

    #[test]
    fn advisor_prompt_all_domains_produce_valid_prompts() {
        let ctx = sample_context();
        for domain in AdvisorDomain::ALL {
            let prompt = advisor_prompt(&ctx, &domain);
            assert!(prompt.contains("Testville"));
            assert!(prompt.len() > 100);
            assert!(
                !prompt.contains("{{"),
                "unreplaced placeholder for {domain:?}"
            );
        }
    }

    #[test]
    fn advisor_prompt_includes_city_statistics() {
        let ctx = sample_context();
        let prompt = advisor_prompt(&ctx, &AdvisorDomain::Economy);
        assert!(prompt.contains("15000"));
    }

    #[test]
    fn alert_prompt_fire() {
        let ctx = sample_context();
        let prompt = alert_prompt(&ctx, &AlertKind::Fire { count: 3 });
        assert!(prompt.contains("3 tiles are currently on fire"));
        assert!(prompt.contains("Testville"));
    }

    #[test]
    fn alert_prompt_deficit() {
        let ctx = sample_context();
        let prompt = alert_prompt(&ctx, &AlertKind::Deficit { treasury: -500 });
        assert!(prompt.contains("$-500"));
    }

    #[test]
    fn alert_prompt_brownout() {
        let ctx = sample_context();
        let prompt = alert_prompt(&ctx, &AlertKind::Brownout);
        assert!(prompt.contains("brownouts"));
    }

    #[test]
    fn alert_prompt_water_shortage() {
        let ctx = sample_context();
        let prompt = alert_prompt(&ctx, &AlertKind::WaterShortage);
        assert!(prompt.contains("water"));
    }

    #[test]
    fn alert_prompt_traffic_crisis() {
        let ctx = sample_context();
        let prompt = alert_prompt(&ctx, &AlertKind::TrafficCrisis);
        assert!(prompt.contains("85%"));
    }

    #[test]
    fn alert_prompt_tornado() {
        let ctx = sample_context();
        let prompt = alert_prompt(&ctx, &AlertKind::Tornado);
        assert!(prompt.contains("tornado"));
    }

    #[test]
    fn alert_prompt_flood() {
        let ctx = sample_context();
        let prompt = alert_prompt(&ctx, &AlertKind::Flood);
        assert!(prompt.contains("flooding"));
    }

    #[test]
    fn alert_prompt_plant_expiring() {
        let ctx = sample_context();
        let prompt = alert_prompt(
            &ctx,
            &AlertKind::PlantExpiring {
                plant_type: "Coal".to_string(),
                months_left: 6,
            },
        );
        assert!(prompt.contains("Coal"));
        assert!(prompt.contains("6 months"));
    }

    #[test]
    fn alert_prompt_has_no_unreplaced_placeholders() {
        let ctx = sample_context();
        let prompt = alert_prompt(&ctx, &AlertKind::Fire { count: 1 });
        assert!(
            !prompt.contains("{{"),
            "unreplaced placeholder in alert prompt"
        );
    }

    #[test]
    fn format_context_shows_power_shortage() {
        let mut ctx = sample_context();
        ctx.power_consumed_mw = 600;
        ctx.power_produced_mw = 400;
        let text = format_context(&ctx);
        assert!(text.contains("SHORTAGE"));
    }

    #[test]
    fn format_context_shows_water_ok() {
        let ctx = sample_context();
        let text = format_context(&ctx);
        // water_produced=200 > water_consumed=150
        assert!(text.contains("(OK)"));
    }

    #[test]
    fn format_context_includes_all_months() {
        for month in 1..=12 {
            let mut ctx = sample_context();
            ctx.month = month;
            let text = format_context(&ctx);
            let month_names = [
                "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
            ];
            assert!(text.contains(month_names[(month - 1) as usize]));
        }
    }

    #[test]
    fn format_context_includes_service_counts() {
        let ctx = sample_context();
        let text = format_context(&ctx);
        assert!(text.contains("2 schools"));
        assert!(text.contains("1 hospitals"));
        assert!(text.contains("3 police"));
        assert!(text.contains("2 fire stations"));
        assert!(text.contains("5 parks"));
    }
}
