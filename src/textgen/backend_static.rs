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

    fn extract_city_name(prompt: &str) -> String {
        for line in prompt.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("You are the editor of The ") {
                if let Some(city) = rest.split(" Daily Tribune").next() {
                    return city.trim().to_string();
                }
            }
            if let Some(rest) = trimmed.strip_prefix("You are the city desk of The ") {
                if let Some(city) = rest.split(" Daily Tribune").next() {
                    return city.trim().to_string();
                }
            }
            if let Some(rest) = trimmed.strip_prefix("City: ") {
                if let Some(city) = rest.split(',').next() {
                    return city.trim().to_string();
                }
            }
        }
        "Testville".to_string()
    }

    fn extract_line<'a>(prompt: &'a str, prefix: &str) -> Option<&'a str> {
        prompt
            .lines()
            .find_map(|line| line.trim().strip_prefix(prefix).map(str::trim))
    }

    fn title_case(text: &str) -> String {
        text.split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    Some(first) => {
                        first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                    }
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn lead_headline(prompt: &str, city_name: &str) -> String {
        let lead_issue = Self::extract_line(prompt, "Lead issue:")
            .unwrap_or("the city is trying to keep its promises straight");
        let lead_lower = lead_issue.to_ascii_lowercase();

        if lead_lower.contains("power") {
            format!("{city_name} Hunts For Enough Power")
        } else if lead_lower.contains("water") {
            format!("{city_name} Races To Keep The Water Flowing")
        } else if lead_lower.contains("budget") || lead_lower.contains("cash") {
            format!("{city_name} City Hall Counts Every Dollar")
        } else if lead_lower.contains("commute") || lead_lower.contains("trip") {
            format!("{city_name} Searches For A Smoother Commute")
        } else if lead_lower.contains("crime") {
            format!("{city_name} Debates How To Restore Calm")
        } else if lead_lower.contains("pollution") {
            format!("{city_name} Looks For Cleaner Air")
        } else if lead_lower.contains("growing") || lead_lower.contains("growth") {
            format!("{city_name} Growth Pushes City Hall To Keep Pace")
        } else {
            format!(
                "{city_name} Faces {}",
                Self::title_case(lead_issue).trim_end_matches('.')
            )
        }
    }

    fn build_newspaper_headlines(prompt: &str) -> String {
        let city_name = Self::extract_city_name(prompt);
        let lead_issue = Self::extract_line(prompt, "Lead issue:")
            .unwrap_or("City Hall is juggling a dozen priorities");
        let transit = Self::extract_line(prompt, "Transit:")
            .unwrap_or("trip success is steady enough to keep the buses moving");
        let services = Self::extract_line(prompt, "Services:")
            .unwrap_or("civic crews are still learning the map");

        [
            format!(
                "{} Lead: {}",
                city_name,
                Self::title_case(lead_issue).trim_end_matches('.')
            ),
            format!("{city_name} Hall Studies The Books As Growth Continues"),
            format!("{city_name} Transit Desk Notes {transit}"),
            format!("{city_name} Neighborhoods Gossip About {services}"),
        ]
        .join("\n")
    }

    fn build_newspaper_article(prompt: &str) -> String {
        let city_name = Self::extract_city_name(prompt);
        let lead_issue = Self::extract_line(prompt, "Lead issue:")
            .unwrap_or("the city is balancing optimism with a few headaches");
        let growth = Self::extract_line(prompt, "Growth:")
            .unwrap_or("population is steady and the moving vans are not panicking");
        let budget = Self::extract_line(prompt, "Budget:")
            .unwrap_or("the treasury is behaving itself for now");
        let utilities = Self::extract_line(prompt, "Utilities:")
            .unwrap_or("the utility boards have no reason to hide under their desks");
        let transit = Self::extract_line(prompt, "Transit:")
            .unwrap_or("trip success is respectable enough to keep commuters hopeful");
        let environment = Self::extract_line(prompt, "Environment:")
            .unwrap_or("the city feels neither sparkling nor doomed");
        let demand = Self::extract_line(prompt, "Demand:")
            .unwrap_or("builders are reading the tea leaves carefully");
        let services = Self::extract_line(prompt, "Services:")
            .unwrap_or("civic services are present and counting clipboards");
        let civic_brag = Self::extract_line(prompt, "Civic brag:")
            .unwrap_or("the city still believes it is one ribbon-cutting away from greatness");
        let resident_gripe = Self::extract_line(prompt, "Resident gripe:")
            .unwrap_or("residents cannot decide whether City Hall is unlucky or simply dramatic");
        let sponsor_angle = Self::extract_line(prompt, "Sponsor angle:")
            .unwrap_or("local advertisers are leaning hard into optimism and folding chairs");
        let street_mood = Self::extract_line(prompt, "Street mood:")
            .unwrap_or("the mood on the sidewalks is hopeful, nosy, and lightly caffeinated");
        let headline = Self::lead_headline(prompt, &city_name);

        format!(
            "PAGE 1: FRONT PAGE\n\
             SECTION: LEAD STORY\n\
             {headline}\n\
             {city_name} enters the month with one big question hanging over City Hall: {lead_issue}. {growth}.\n\n\
             At the same time, {budget}. Department chiefs say {utilities}, while the traffic desk notes that {transit}.\n\n\
             SECTION: CITY BEAT\n\
             Around town, the city beat keeps circling the same civic rumor: {services}. Neighbors add that {demand}, which is either a sign of confidence or proof that nobody can stop talking at zoning meetings.\n\n\
             SECTION: CITY OWNER'S ADVERTISEMENT\n\
             Paid for by the Office of Extremely Confident Civic Promotion.\n\
             Come see the future of {city_name}, now with extra speeches and fewer apologies.\n\
             {civic_brag}.\n\
             City Hall promises that tomorrow's progress will arrive shortly after the next committee meeting.\n\n\
             PAGE 2: READER FORUM\n\
             SECTION: LETTERS FROM READERS\n\
             Dear Tribune,\n\
             If the city can survive {lead_issue}, surely it can survive one more complaint from me. Signed, A Taxpayer Who Keeps The Receipts.\n\n\
             Dear Tribune,\n\
             {street_mood}, and I would simply like the buses, the lights, and my patience to arrive in the same week. Signed, Someone Waiting By The Curb.\n\n\
             SECTION: SIDEWALK QUOTE\n\
             \"I knew the city was growing when even the pigeons started acting like property speculators.\" — A resident near the busiest corner in town.\n\n\
             SECTION: EDITORIAL\n\
             Dear Mayor,\n\
             Pick one promise and finish it before the next parade of complaints arrives. {environment}, and citizens can tell when City Hall is solving problems versus merely naming them.\n\n\
             PAGE 3: MARKET SQUARE\n\
             SECTION: SHOPKEEPER SPOTLIGHT\n\
             Visit Main Street Mercantile, where we proudly {sponsor_angle}. Management insists that a thriving city begins with sturdy shelves and perfectly timed optimism.\n\n\
             SECTION: CONTACT ADS\n\
             SEEKING: Charming lunch companion who understands municipal drama and still believes in clean sidewalks.\n\
             LONELY HEART: Ambitious commuter seeks fellow dreamer with strong opinions about buses and pastry.\n\
             NEW IN TOWN: Recent arrival hopes to meet someone who can explain zoning with compassion and maybe dessert.\n\n\
             SECTION: CLASSIFIEDS\n\
             WANTED: One practical planner who can read {lead_issue} without fainting.\n\
             FOR SALE: Slightly used ribbon-cutting scissors, ideal for the next municipal triumph.\n\
             NOTICE: Residents are reminded that arguing about roads still counts as a local pastime.\n\n\
             PAGE 4: FEATURES & FUN\n\
             SECTION: JOKE CORNER\n\
             Why did the mayor bring a ladder to City Hall? Because everyone kept saying the city needed higher standards.\n\
             What do you call a perfectly timed bus in {city_name}? A rumor with headlights.\n\n\
             SECTION: COMMUNITY CALENDAR\n\
             MONDAY: Neighborhood association meets to discuss {resident_gripe} and the urgent shortage of patience.\n\
             WEDNESDAY: Chamber of commerce luncheon on how {civic_brag} can somehow be monetized.\n\
             SATURDAY: Free park concert, weather and paperwork permitting.\n\n\
             SECTION: WEATHER DESK\n\
             Expect a bright civic morning followed by scattered paperwork. The forecast improves noticeably whenever the utilities and the treasury stop glaring at each other."
        )
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
        let text = if prompt.contains("LEAD STORY") || prompt.contains("CLASSIFIEDS") {
            Self::build_newspaper_article(prompt)
        } else if prompt.starts_with("City names:") || prompt.contains("city name") {
            self.pick(CITY_NAMES).to_string()
        } else if prompt.contains("editor")
            || prompt.contains("newspaper")
            || prompt.contains("Tribune")
        {
            Self::build_newspaper_headlines(prompt)
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
    fn generates_grounded_newspaper_article() {
        let mut gen = StaticGenerator::new();
        let prompt = "You are the editor of The Testville Daily Tribune.\nLead issue: the power grid is falling short of demand\nGrowth: population 5000 (+50 this month)\nBudget: treasury $15000; monthly income $200\nUtilities: power supply falls short of demand by 200 MW; water supply is ahead by 50 units\nTransit: trip success is 85%\nEnvironment: pollution 80/255, crime 60/255, fire risk 40/255, land value 120/255\nDemand: strongest residential demand at 0.60\nServices: 2 schools, 1 hospitals, 3 police stations, 2 fire stations, 5 parks\nLEAD STORY\nCLASSIFIEDS";
        let text = gen.generate(prompt, 720, 0.8).unwrap();
        for marker in [
            "PAGE 1: FRONT PAGE",
            "PAGE 2: READER FORUM",
            "PAGE 3: MARKET SQUARE",
            "PAGE 4: FEATURES & FUN",
            "SECTION: LEAD STORY",
            "SECTION: CITY BEAT",
            "SECTION: CITY OWNER'S ADVERTISEMENT",
            "SECTION: LETTERS FROM READERS",
            "SECTION: SIDEWALK QUOTE",
            "SECTION: EDITORIAL",
            "SECTION: SHOPKEEPER SPOTLIGHT",
            "SECTION: CONTACT ADS",
            "SECTION: CLASSIFIEDS",
            "SECTION: JOKE CORNER",
            "SECTION: COMMUNITY CALENDAR",
            "SECTION: WEATHER DESK",
        ] {
            assert!(text.contains(marker), "missing marker {marker}");
        }
        assert!(text.contains("Testville"));
        assert!(text.contains("power"));
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
