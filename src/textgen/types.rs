/// Task variants that can be submitted to the LLM worker thread.
#[derive(Debug, Clone)]
pub enum LlmTask {
    GenerateCityName,
    WriteNewspaper {
        context: CityContext,
    },
    AdvisorAdvice {
        context: CityContext,
        domain: AdvisorDomain,
    },
    GenerateAlert {
        context: CityContext,
        alert_kind: AlertKind,
    },
    WriteNewspaperArticle {
        context: CityContext,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvisorDomain {
    Economy,
    CityPlanning,
    Education,
    Safety,
    Transport,
}

impl AdvisorDomain {
    pub const ALL: [AdvisorDomain; 5] = [
        AdvisorDomain::Economy,
        AdvisorDomain::CityPlanning,
        AdvisorDomain::Education,
        AdvisorDomain::Safety,
        AdvisorDomain::Transport,
    ];

    pub fn label(self) -> &'static str {
        match self {
            AdvisorDomain::Economy => "Economy",
            AdvisorDomain::CityPlanning => "City Planning",
            AdvisorDomain::Education => "Education",
            AdvisorDomain::Safety => "Safety",
            AdvisorDomain::Transport => "Transport",
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(not(test), allow(dead_code))]
pub enum AlertKind {
    Fire {
        count: usize,
    },
    Deficit {
        treasury: i64,
    },
    Brownout,
    WaterShortage,
    TrafficCrisis,
    Tornado,
    Flood,
    PlantExpiring {
        plant_type: String,
        months_left: u32,
    },
}

/// Compact snapshot of game state for prompt building. No game logic here —
/// just serialized numbers that `prompt.rs` formats into text.
#[derive(Debug, Clone)]
pub struct CityContext {
    pub city_name: String,
    pub year: i32,
    pub month: u8,
    pub population: u64,
    pub treasury: i64,
    pub last_income: i64,
    pub tax_res: u8,
    pub tax_comm: u8,
    pub tax_ind: u8,
    pub demand_res: f32,
    pub demand_comm: f32,
    pub demand_ind: f32,
    pub power_produced_mw: u32,
    pub power_consumed_mw: u32,
    pub water_produced: u32,
    pub water_consumed: u32,
    pub avg_pollution: u8,
    pub avg_crime: u8,
    pub avg_land_value: u8,
    pub avg_fire_risk: u8,
    pub active_fires: usize,
    pub trip_success_rate: f32,
    pub pop_delta: i64,
    pub num_schools: u32,
    pub num_hospitals: u32,
    pub num_police: u32,
    pub num_fire_stations: u32,
    pub num_parks: u32,
}

/// Response returned from the LLM worker thread.
#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub task_tag: LlmTaskTag,
    pub text: String,
}

/// Lightweight discriminant so callers know which request a response answers
/// without cloning the full task payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmTaskTag {
    CityName,
    Newspaper,
    Advisor(AdvisorDomain),
    Alert,
    NewspaperArticle,
}

impl LlmTask {
    pub fn tag(&self) -> LlmTaskTag {
        match self {
            LlmTask::GenerateCityName => LlmTaskTag::CityName,
            LlmTask::WriteNewspaper { .. } => LlmTaskTag::Newspaper,
            LlmTask::AdvisorAdvice { domain, .. } => LlmTaskTag::Advisor(*domain),
            LlmTask::GenerateAlert { .. } => LlmTaskTag::Alert,
            LlmTask::WriteNewspaperArticle { .. } => LlmTaskTag::NewspaperArticle,
        }
    }
}

#[cfg(test)]
pub fn sample_context() -> CityContext {
    CityContext {
        city_name: "Testville".to_string(),
        year: 1950,
        month: 6,
        population: 5000,
        treasury: 15000,
        last_income: 200,
        tax_res: 9,
        tax_comm: 9,
        tax_ind: 9,
        demand_res: 0.6,
        demand_comm: 0.3,
        demand_ind: 0.4,
        power_produced_mw: 500,
        power_consumed_mw: 300,
        water_produced: 200,
        water_consumed: 150,
        avg_pollution: 80,
        avg_crime: 60,
        avg_land_value: 120,
        avg_fire_risk: 40,
        active_fires: 0,
        trip_success_rate: 0.85,
        pop_delta: 50,
        num_schools: 2,
        num_hospitals: 1,
        num_police: 3,
        num_fire_stations: 2,
        num_parks: 5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_tag_city_name() {
        assert_eq!(LlmTask::GenerateCityName.tag(), LlmTaskTag::CityName);
    }

    #[test]
    fn task_tag_newspaper() {
        let task = LlmTask::WriteNewspaper {
            context: sample_context(),
        };
        assert_eq!(task.tag(), LlmTaskTag::Newspaper);
    }

    #[test]
    fn task_tag_advisor_preserves_domain() {
        for domain in AdvisorDomain::ALL {
            let task = LlmTask::AdvisorAdvice {
                context: sample_context(),
                domain,
            };
            assert_eq!(task.tag(), LlmTaskTag::Advisor(domain));
        }
    }

    #[test]
    fn task_tag_alert() {
        let task = LlmTask::GenerateAlert {
            context: sample_context(),
            alert_kind: AlertKind::Fire { count: 3 },
        };
        assert_eq!(task.tag(), LlmTaskTag::Alert);
    }

    #[test]
    fn advisor_domain_all_has_five_entries() {
        assert_eq!(AdvisorDomain::ALL.len(), 5);
    }

    #[test]
    fn advisor_domain_labels_are_non_empty() {
        for domain in AdvisorDomain::ALL {
            assert!(!domain.label().is_empty());
        }
    }

    #[test]
    fn advisor_domain_labels_are_unique() {
        let labels: Vec<&str> = AdvisorDomain::ALL.iter().map(|d| d.label()).collect();
        for (i, a) in labels.iter().enumerate() {
            for b in &labels[i + 1..] {
                assert_ne!(a, b, "duplicate advisor label");
            }
        }
    }

    #[test]
    fn llm_response_carries_tag_and_text() {
        let response = LlmResponse {
            task_tag: LlmTaskTag::CityName,
            text: "Springfield".to_string(),
        };
        assert_eq!(response.task_tag, LlmTaskTag::CityName);
        assert_eq!(response.text, "Springfield");
    }

    #[test]
    fn city_context_clone_is_independent() {
        let ctx = sample_context();
        let mut cloned = ctx.clone();
        cloned.city_name = "Othertown".to_string();
        cloned.population = 999;
        assert_eq!(ctx.city_name, "Testville");
        assert_eq!(ctx.population, 5000);
    }
}
