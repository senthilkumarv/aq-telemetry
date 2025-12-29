// Aquarium domain model
use telemetry_thrift::SDAquarium;

#[derive(Debug, Clone)]
pub struct Aquarium {
    pub id: String,
    pub name: String,
}

impl Aquarium {
    pub fn new(id: String) -> Self {
        let name = Self::format_name(&id);
        Self { id, name }
    }

    fn format_name(id: &str) -> String {
        // Convert "Great_Barrier_" to "Great Barrier"
        id.trim_end_matches('_').replace('_', " ")
    }

    pub fn to_thrift(&self) -> SDAquarium {
        SDAquarium::new(Some(self.id.clone()), Some(self.name.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_name() {
        let aquarium = Aquarium::new("Great_Barrier_".to_string());
        assert_eq!(aquarium.name, "Great Barrier");

        let aquarium = Aquarium::new("Planet_72".to_string());
        assert_eq!(aquarium.name, "Planet 72");
    }
}

