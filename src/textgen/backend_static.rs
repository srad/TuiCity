use super::generator::TextGenerator;

const CITY_NAMES: &[&str] = &[
    "Riverside",
    "Oakdale",
    "Fairview",
    "Pinecrest",
    "Maplewood",
    "Cedar Falls",
    "Brookhaven",
    "Willowdale",
    "Stonebridge",
    "Lakewood",
    "Clearwater",
    "Springdale",
    "Ashford",
    "Greenfield",
    "Thornton",
    "Ridgemont",
    "Sunset Bay",
    "Windermere",
    "Harborview",
    "Crestwood",
    "Millbrook",
    "Foxborough",
    "Westfield",
    "Kingsport",
    "Whitewood",
    "Easton",
    "Northgate",
    "Southdale",
    "Hillcrest",
    "Bayshore",
    "Glendale",
    "Belmont",
    "Hawthorne",
    "Silverdale",
    "Rosewood",
    "Edgewood",
    "Elmhurst",
    "Brighton",
    "Clarkston",
    "Waverly",
    "Lakeshore",
    "Granville",
    "Fairmont",
    "Hartford",
    "Dunmore",
    "Riverdale",
    "Fernwood",
    "Stillwater",
    "Briarwood",
    "Summitville",
];

const NEWSPAPER_HEADLINES: &[&str] = &[
    "City council debates new zoning regulations.",
    "Local businesses report steady growth this quarter.",
    "Residents rally for improved park facilities.",
    "Traffic study reveals congestion hotspots downtown.",
    "New development project breaks ground on the west side.",
    "School board announces plans for education expansion.",
    "Water department upgrades aging infrastructure.",
    "Community groups organize neighborhood cleanup drive.",
    "Mayor addresses concerns about rising utility costs.",
    "Fire department celebrates year without major incidents.",
];

const ADVISOR_ECONOMY: &[&str] = &[
    "Consider adjusting tax rates to balance revenue with growth demand.",
    "Monitor the treasury closely and avoid overextending on infrastructure.",
    "A healthy commercial sector will boost tax income over time.",
];

const ADVISOR_PLANNING: &[&str] = &[
    "Balance residential, commercial, and industrial zones for stable growth.",
    "Parks and green spaces improve land value in surrounding areas.",
    "Avoid clustering heavy industry near residential neighborhoods.",
];

const ADVISOR_EDUCATION: &[&str] = &[
    "Schools increase land value and attract families to nearby zones.",
    "Libraries complement schools and provide additional education coverage.",
    "Spread educational facilities evenly across the city.",
];

const ADVISOR_SAFETY: &[&str] = &[
    "Police stations reduce crime in their coverage radius.",
    "Fire stations lower fire risk and improve emergency response.",
    "Ensure coverage overlaps to avoid gaps in service areas.",
];

const ADVISOR_TRANSPORT: &[&str] = &[
    "A connected road network is essential for trip success.",
    "Transit options like buses and rail reduce road congestion.",
    "Watch the trip success rate as a key indicator of network health.",
];

const ALERT_TEMPLATES: &[&str] = &[
    "Citizens are concerned about the current situation. City officials are working on a response.",
    "The mayor has called an emergency meeting to address the crisis.",
    "Emergency services are mobilizing to handle the situation.",
];

pub struct StaticGenerator {
    counter: usize,
}

impl StaticGenerator {
    pub fn new() -> Self {
        Self { counter: 0 }
    }

    fn pick<'a>(&mut self, list: &'a [&str]) -> &'a str {
        let idx = self.counter % list.len();
        self.counter = self.counter.wrapping_add(1);
        list[idx]
    }
}

impl TextGenerator for StaticGenerator {
    fn generate(
        &mut self,
        prompt: &str,
        _max_tokens: usize,
        _temperature: f32,
    ) -> Result<String, String> {
        // Detect which kind of prompt this is by checking for known patterns.
        let text = if prompt.starts_with("City names:") || prompt.contains("city name") {
            self.pick(CITY_NAMES).to_string()
        } else if prompt.contains("editor")
            || prompt.contains("newspaper")
            || prompt.contains("Tribune")
        {
            // Newspaper — return a few headlines
            let h1 = self.pick(NEWSPAPER_HEADLINES);
            let h2 = self.pick(NEWSPAPER_HEADLINES);
            format!("{h1}\n{h2}")
        } else if prompt.contains("Finance Advisor") || prompt.contains("treasury health") {
            self.pick(ADVISOR_ECONOMY).to_string()
        } else if prompt.contains("City Planning Advisor") || prompt.contains("zoning balance") {
            self.pick(ADVISOR_PLANNING).to_string()
        } else if prompt.contains("Education Advisor") || prompt.contains("school coverage") {
            self.pick(ADVISOR_EDUCATION).to_string()
        } else if prompt.contains("Safety Advisor") || prompt.contains("crime levels") {
            self.pick(ADVISOR_SAFETY).to_string()
        } else if prompt.contains("Transport Advisor") || prompt.contains("trip success") {
            self.pick(ADVISOR_TRANSPORT).to_string()
        } else if prompt.contains("alert")
            || prompt.contains("emergency")
            || prompt.contains("situation")
        {
            self.pick(ALERT_TEMPLATES).to_string()
        } else {
            // Generic fallback
            self.pick(CITY_NAMES).to_string()
        };

        Ok(text)
    }

    fn backend_name(&self) -> &str {
        "static"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_city_name() {
        let mut gen = StaticGenerator::new();
        let result = gen.generate("City names: Riverside, Oakdale,", 3, 1.0);
        assert!(result.is_ok());
        let name = result.unwrap();
        assert!(!name.is_empty());
        assert!(CITY_NAMES.contains(&name.as_str()));
    }

    #[test]
    fn generates_different_names_on_successive_calls() {
        let mut gen = StaticGenerator::new();
        let a = gen.generate("City names:", 3, 1.0).unwrap();
        let b = gen.generate("City names:", 3, 1.0).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn generates_newspaper() {
        let mut gen = StaticGenerator::new();
        let result = gen.generate("The Testville Daily Tribune — Headlines", 300, 0.7);
        assert!(result.is_ok());
        let text = result.unwrap();
        assert!(text.contains('\n')); // Multiple headlines
    }

    #[test]
    fn generates_advisor_economy() {
        let mut gen = StaticGenerator::new();
        let result = gen.generate("You are the Finance Advisor", 250, 0.6);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn generates_alert() {
        let mut gen = StaticGenerator::new();
        let result = gen.generate("Write an alert about the situation", 60, 0.8);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn backend_name_is_static() {
        let gen = StaticGenerator::new();
        assert_eq!(gen.backend_name(), "static");
    }
}
