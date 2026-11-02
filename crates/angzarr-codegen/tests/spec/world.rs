//! The generator a scenario runs: the protos it declares, compiled, then
//! linted (every check generation makes: the descriptors, the domain
//! layer, the scaffold) or generated from, and what came of it.

use angzarr_codegen::{
    CodegenError, Descriptor, Translate, adapters, descriptors, markup_domain, scaffold,
};
use angzarr_proto::v1::ComponentDescriptor;
use prost::Message as _;

use crate::protos::{Protos, compile, full, package};

/// The version every scenario's build is at.
pub const VERSION: &str = "1.4.2+build.7";

#[derive(Debug, Default, cucumber::World)]
pub struct Generator {
    pub protos: Protos,
    /// Each component by the name a scenario gives it, its full name.
    pub names: std::collections::BTreeMap<String, String>,
    /// The policy translate selections generation runs with.
    pub translate: Vec<Translate>,
    /// What linting or generating came to.
    pub outcome: Option<Result<Generated, CodegenError>>,
    /// The build's version, if the scenario names one.
    pub version: Option<String>,
    /// The component declared last, by the name the scenario gave it.
    pub last_component: Option<String>,
    /// The input declared last, by its full name.
    pub last_input: Option<String>,
}

/// What generation made.
#[derive(Debug, Default)]
pub struct Generated {
    pub descriptors: Vec<Descriptor>,
    /// The generated adapters: the markup each component's handlers carry.
    pub adapters: String,
}

impl Generator {
    /// A component `name` of `kind` owning `domain`.
    pub fn component(
        &mut self,
        name: &str,
        kind: &str,
        domain: &str,
    ) -> &mut crate::protos::Component {
        let full = format!("{}.{name}", package(domain));
        self.names.insert(name.to_string(), full.clone());
        self.last_component = Some(name.to_string());
        let message = self.protos.message(&full);
        message
            .component
            .get_or_insert_with(|| crate::protos::Component {
                kind: kind.to_string(),
                ..Default::default()
            })
    }

    /// The full name of the component a scenario names.
    pub fn named(&self, name: &str) -> String {
        self.names.get(name).cloned().unwrap_or_else(|| full(name))
    }

    /// The input `name` (`order.ShipOrder`) for the component `component`.
    pub fn input(&mut self, name: &str, component: &str) -> &mut crate::protos::Input {
        let component = self.named(component);
        self.last_input = Some(full(name));
        let message = self.protos.message(&full(name));
        message.input.get_or_insert_with(|| crate::protos::Input {
            component,
            ..Default::default()
        })
    }

    /// The protos compiled, and every generation check run on them.
    pub fn lint(&mut self) {
        let files = self.protos.files();
        let set = compile(&files);
        let names: Vec<&str> = files.keys().map(String::as_str).collect();
        let wire: Vec<(String, String)> = files
            .keys()
            .map(|f| {
                (
                    f.trim_end_matches(".proto").to_string(),
                    "crate::wire".to_string(),
                )
            })
            .collect();
        let wire: Vec<(&str, &str)> = wire.iter().map(|(p, w)| (p.as_str(), w.as_str())).collect();
        let version = self.version.clone().unwrap_or_else(|| VERSION.to_string());
        let outcome = descriptors(&set, &names, &version).and_then(|descriptors| {
            markup_domain(&set, &names, &wire)?;
            // The user's files `angzarr scaffold rust` writes: their checks.
            scaffold(&set, &names, &self.translate)?;
            let adapters = adapters(&set, &names, &[], &|_| None)?;
            Ok(Generated {
                descriptors,
                adapters,
            })
        });
        self.outcome = Some(outcome);
    }

    /// What generation made; panics if it refused.
    pub fn generated(&self) -> &Generated {
        match self.outcome.as_ref().expect("generated") {
            Ok(generated) => generated,
            Err(error) => panic!("generation refused: {error:?}"),
        }
    }

    /// Why generation refused; panics if it did not.
    pub fn refused(&self) -> &CodegenError {
        match self.outcome.as_ref().expect("linted") {
            Err(error) => error,
            Ok(_) => panic!("generation did not refuse"),
        }
    }

    /// The only component's descriptor.
    pub fn only_descriptor(&self) -> ComponentDescriptor {
        let all = &self.generated().descriptors;
        assert_eq!(all.len(), 1, "{all:?}");
        ComponentDescriptor::decode(all[0].encoded.as_slice()).expect("an encoded descriptor")
    }
}
