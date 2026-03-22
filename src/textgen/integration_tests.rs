#[cfg(test)]
mod tests {
    use crate::textgen::{
        backend_static::StaticGenerator,
        generator::TextGenerator,
        types::{AdvisorDomain, AlertKind, CityContext, LlmTask, LlmTaskTag},
        TextGenService,
    };
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    fn sample_context() -> CityContext {
        crate::textgen::types::sample_context()
    }

    fn poll_response(service: &TextGenService, timeout: Duration) -> Option<crate::textgen::types::LlmResponse> {
        let start = Instant::now();
        loop {
            if let Some(resp) = service.poll() {
                return Some(resp);
            }
            if start.elapsed() > timeout {
                return None;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn wait_for_backend(service: &TextGenService) {
        let start = Instant::now();
        while service.backend_name() == "initializing" {
            if start.elapsed() > Duration::from_secs(5) {
                panic!("timeout waiting for backend init");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    // ── Static generator direct tests ──────────────────────────────

    #[test]
    fn static_city_name() {
        let mut gen = StaticGenerator::new();
        let result = gen.generate("City names: Riverside,", 3, 1.0).unwrap();
        assert!(!result.is_empty());
        assert!(!result.contains('\n'));
    }

    #[test]
    fn static_city_name_varies() {
        let mut gen = StaticGenerator::new();
        let a = gen.generate("City names:", 3, 1.0).unwrap();
        let b = gen.generate("City names:", 3, 1.0).unwrap();
        let c = gen.generate("City names:", 3, 1.0).unwrap();
        // At least two of three should differ
        assert!(a != b || b != c, "expected variation across calls");
    }

    #[test]
    fn static_newspaper() {
        let mut gen = StaticGenerator::new();
        let result = gen.generate("The Testville Daily Tribune — Headlines", 300, 0.7).unwrap();
        assert!(!result.is_empty());
        assert!(result.contains('\n'), "newspaper should have multiple headlines");
    }

    #[test]
    fn static_advisor_all_domains() {
        let mut gen = StaticGenerator::new();
        let prompts = [
            "Finance Advisor report:",
            "City Planning Advisor report:",
            "Education Advisor report:",
            "Safety Advisor report:",
            "Transport Advisor report:",
        ];
        for prompt in prompts {
            let result = gen.generate(prompt, 250, 0.6).unwrap();
            assert!(!result.is_empty(), "empty response for {prompt}");
        }
    }

    #[test]
    fn static_alert() {
        let mut gen = StaticGenerator::new();
        let result = gen.generate("BREAKING NEWS — alert situation", 60, 0.8).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn static_unknown_prompt_returns_something() {
        let mut gen = StaticGenerator::new();
        let result = gen.generate("completely unknown input", 10, 1.0).unwrap();
        assert!(!result.is_empty());
    }

    // ── Static backend via TextGenService ──────────────────────────

    #[test]
    fn service_static_city_name() {
        let service = TextGenService::start(PathBuf::from("/nonexistent"));
        wait_for_backend(&service);
        assert_eq!(service.backend_name(), "static");

        service.request(LlmTask::GenerateCityName);
        let resp = poll_response(&service, Duration::from_secs(2)).expect("timeout");
        assert_eq!(resp.task_tag, LlmTaskTag::CityName);
        assert!(!resp.text.is_empty());
    }

    #[test]
    fn service_static_newspaper() {
        let service = TextGenService::start(PathBuf::from("/nonexistent"));
        wait_for_backend(&service);

        service.request(LlmTask::WriteNewspaper {
            context: sample_context(),
        });
        let resp = poll_response(&service, Duration::from_secs(2)).expect("timeout");
        assert_eq!(resp.task_tag, LlmTaskTag::Newspaper);
        assert!(!resp.text.is_empty());
    }

    #[test]
    fn service_static_all_advisors() {
        let service = TextGenService::start(PathBuf::from("/nonexistent"));
        wait_for_backend(&service);

        for domain in AdvisorDomain::ALL {
            service.request(LlmTask::AdvisorAdvice {
                context: sample_context(),
                domain,
            });
            let resp = poll_response(&service, Duration::from_secs(2))
                .unwrap_or_else(|| panic!("timeout for {domain:?}"));
            assert_eq!(resp.task_tag, LlmTaskTag::Advisor(domain));
            assert!(!resp.text.is_empty(), "empty for {domain:?}");
        }
    }

    #[test]
    fn service_static_all_alert_kinds() {
        let service = TextGenService::start(PathBuf::from("/nonexistent"));
        wait_for_backend(&service);

        let alerts: Vec<AlertKind> = vec![
            AlertKind::Fire { count: 5 },
            AlertKind::Deficit { treasury: -200 },
            AlertKind::Brownout,
            AlertKind::WaterShortage,
            AlertKind::TrafficCrisis,
            AlertKind::Tornado,
            AlertKind::Flood,
            AlertKind::PlantExpiring {
                plant_type: "Coal".to_string(),
                months_left: 6,
            },
        ];

        for alert_kind in alerts {
            service.request(LlmTask::GenerateAlert {
                context: sample_context(),
                alert_kind: alert_kind.clone(),
            });
            let resp = poll_response(&service, Duration::from_secs(2))
                .unwrap_or_else(|| panic!("timeout for {alert_kind:?}"));
            assert_eq!(resp.task_tag, LlmTaskTag::Alert);
            assert!(!resp.text.is_empty(), "empty for {alert_kind:?}");
        }
    }

    // ── llama.cpp backend via TextGenService (requires model) ─────

    #[cfg(feature = "llm")]
    fn start_llm_service() -> Option<TextGenService> {
        let model_dir = crate::textgen::default_model_dir();
        if !crate::textgen::download::model_files_present(&model_dir) {
            return None;
        }
        let service = TextGenService::start(model_dir);
        let start = Instant::now();
        while service.backend_name() == "initializing" {
            if start.elapsed() > Duration::from_secs(30) {
                return None;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        if service.has_model() {
            Some(service)
        } else {
            None
        }
    }

    #[cfg(feature = "llm")]
    #[test]
    fn llm_backend_generates_text() {
        let service = match start_llm_service() {
            Some(s) => s,
            None => {
                eprintln!("Skipping: no llama.cpp model available");
                return;
            }
        };
        assert_eq!(service.backend_name(), "llama.cpp");

        service.request(LlmTask::GenerateCityName);
        let resp = poll_response(&service, Duration::from_secs(30)).expect("timeout");
        assert_eq!(resp.task_tag, LlmTaskTag::CityName);
        assert!(!resp.text.is_empty());
        eprintln!("City name: {}", resp.text);
    }

    #[cfg(feature = "llm")]
    #[test]
    fn llm_backend_newspaper() {
        let service = match start_llm_service() {
            Some(s) => s,
            None => {
                eprintln!("Skipping: no llama.cpp model available");
                return;
            }
        };

        service.request(LlmTask::WriteNewspaper {
            context: sample_context(),
        });
        let resp = poll_response(&service, Duration::from_secs(60)).expect("timeout");
        assert_eq!(resp.task_tag, LlmTaskTag::Newspaper);
        assert!(!resp.text.is_empty());
        eprintln!("Newspaper:\n{}", resp.text);
    }

    #[cfg(feature = "llm")]
    #[test]
    fn llm_backend_advisor() {
        let service = match start_llm_service() {
            Some(s) => s,
            None => {
                eprintln!("Skipping: no llama.cpp model available");
                return;
            }
        };

        service.request(LlmTask::AdvisorAdvice {
            context: sample_context(),
            domain: AdvisorDomain::Economy,
        });
        let resp = poll_response(&service, Duration::from_secs(60)).expect("timeout");
        assert_eq!(resp.task_tag, LlmTaskTag::Advisor(AdvisorDomain::Economy));
        assert!(!resp.text.is_empty());
        eprintln!("Advisor:\n{}", resp.text);
    }

    #[cfg(feature = "llm")]
    #[test]
    fn llm_backend_alert() {
        let service = match start_llm_service() {
            Some(s) => s,
            None => {
                eprintln!("Skipping: no llama.cpp model available");
                return;
            }
        };

        service.request(LlmTask::GenerateAlert {
            context: sample_context(),
            alert_kind: AlertKind::Fire { count: 3 },
        });
        let resp = poll_response(&service, Duration::from_secs(60)).expect("timeout");
        assert_eq!(resp.task_tag, LlmTaskTag::Alert);
        assert!(!resp.text.is_empty());
        eprintln!("Alert: {}", resp.text);
    }
}
