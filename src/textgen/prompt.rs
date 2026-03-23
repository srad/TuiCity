use super::types::{AdvisorDomain, AlertKind, CityContext};

// Templates loaded at compile time from assets/prompts/.
const TMPL_CITY_NAME: &str = include_str!("../../assets/prompts/city_name.txt");
const TMPL_NEWSPAPER: &str = include_str!("../../assets/prompts/newspaper.txt");
const TMPL_ADVISOR: &str = include_str!("../../assets/prompts/advisor.txt");
const TMPL_ALERT: &str = include_str!("../../assets/prompts/alert.txt");
const TMPL_NEWSPAPER_ARTICLE: &str =
    include_str!("../../assets/prompts/newspaper_article.txt");
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

fn format_context(ctx: &CityContext) -> String {
    let month_name = match ctx.month {
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
    };

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
        month = month_name,
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

pub fn city_name_prompt() -> String {
    TMPL_CITY_NAME.to_string()
}

pub fn newspaper_article_prompt(ctx: &CityContext) -> String {
    let month_name = match ctx.month {
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
    };
    render(
        TMPL_NEWSPAPER_ARTICLE,
        &[
            ("city_name", &ctx.city_name),
            ("month_name", month_name),
            ("year", &ctx.year.to_string()),
            ("city_status", &format_context(ctx)),
        ],
    )
}

pub fn newspaper_prompt(ctx: &CityContext) -> String {
    render(
        TMPL_NEWSPAPER,
        &[
            ("city_name", &ctx.city_name),
            ("population", &ctx.population.to_string()),
            ("treasury", &ctx.treasury.to_string()),
            ("city_status", &format_context(ctx)),
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
