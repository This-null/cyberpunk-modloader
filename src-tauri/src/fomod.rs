use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
enum Dep {
    Flag(String, String),
    File(String, String),
    Group { and: bool, items: Vec<Dep> },
}

#[derive(Debug, Clone)]
struct Rule {
    source: String,
    dest: String,
    folder: bool,
    priority: i32,
}

#[derive(Debug, Clone)]
struct Plugin {
    name: String,
    description: String,
    image: String,
    files: Vec<Rule>,
    flags: Vec<(String, String)>,
    default_type: String,
    patterns: Vec<(Dep, String)>,
}

#[derive(Debug, Clone)]
struct Group {
    name: String,
    kind: String,
    order: String,
    plugins: Vec<Plugin>,
}

#[derive(Debug, Clone)]
struct Step {
    name: String,
    visible: Option<Dep>,
    order: String,
    groups: Vec<Group>,
}

#[derive(Debug, Clone)]
struct Pattern {
    dep: Dep,
    files: Vec<Rule>,
}

#[derive(Debug, Clone)]
pub struct Config {
    name: String,
    order: String,
    required: Vec<Rule>,
    steps: Vec<Step>,
    patterns: Vec<Pattern>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PluginView {
    pub name: String,
    pub description: String,
    pub image: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct GroupView {
    pub name: String,
    pub kind: String,
    pub order: String,
    pub plugins: Vec<PluginView>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct StepView {
    pub name: String,
    pub order: String,
    pub groups: Vec<GroupView>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct View {
    pub name: String,
    pub order: String,
    pub steps: Vec<StepView>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Evaluation {
    pub visible: Vec<bool>,
    pub types: Vec<Vec<Vec<String>>>,
}

pub type Selection = Vec<Vec<Vec<usize>>>;

pub fn decode(bytes: &[u8]) -> String {
    let utf16 = |data: &[u8], le: bool| {
        let units: Vec<u16> = data
            .chunks_exact(2)
            .map(|c| if le { u16::from_le_bytes([c[0], c[1]]) } else { u16::from_be_bytes([c[0], c[1]]) })
            .collect();
        String::from_utf16_lossy(&units)
    };
    match bytes {
        [0xFF, 0xFE, rest @ ..] => utf16(rest, true),
        [0xFE, 0xFF, rest @ ..] => utf16(rest, false),
        [0xEF, 0xBB, 0xBF, rest @ ..] => String::from_utf8_lossy(rest).into_owned(),
        _ => String::from_utf8_lossy(bytes).into_owned(),
    }
}

fn is(node: &roxmltree::Node, name: &str) -> bool {
    node.is_element() && node.tag_name().name().eq_ignore_ascii_case(name)
}

fn kids<'a, 'b: 'a>(node: roxmltree::Node<'a, 'b>, name: &'static str) -> impl Iterator<Item = roxmltree::Node<'a, 'b>> {
    node.children().filter(move |c| is(c, name))
}

fn kid<'a, 'b: 'a>(node: roxmltree::Node<'a, 'b>, name: &'static str) -> Option<roxmltree::Node<'a, 'b>> {
    kids(node, name).next()
}

fn attr(node: &roxmltree::Node, name: &str) -> String {
    node.attributes().find(|a| a.name().eq_ignore_ascii_case(name)).map(|a| a.value().to_string()).unwrap_or_default()
}

fn text(node: Option<roxmltree::Node>) -> String {
    node.map(|n| n.text().unwrap_or_default().trim().to_string()).unwrap_or_default()
}

fn parse_dep(node: roxmltree::Node) -> Dep {
    let and = !attr(&node, "operator").eq_ignore_ascii_case("or");
    let mut items = Vec::new();
    for c in node.children().filter(|c| c.is_element()) {
        if is(&c, "flagDependency") {
            items.push(Dep::Flag(attr(&c, "flag"), attr(&c, "value")));
        } else if is(&c, "fileDependency") {
            items.push(Dep::File(attr(&c, "file"), attr(&c, "state")));
        } else if is(&c, "dependencies") {
            items.push(parse_dep(c));
        }
    }
    Dep::Group { and, items }
}

fn parse_rules(node: roxmltree::Node) -> Vec<Rule> {
    node.children()
        .filter(|c| is(c, "file") || is(c, "folder"))
        .map(|c| {
            let source = attr(&c, "source");
            let dest = c.attributes().find(|a| a.name().eq_ignore_ascii_case("destination")).map(|a| a.value().to_string());
            let folder = is(&c, "folder");
            Rule { dest: dest.unwrap_or_else(|| if folder { String::new() } else { source.clone() }), source, folder, priority: attr(&c, "priority").trim().parse().unwrap_or(0) }
        })
        .collect()
}

fn parse_plugin(node: roxmltree::Node) -> Plugin {
    let mut plugin = Plugin {
        name: attr(&node, "name"),
        description: text(kid(node, "description")),
        image: kid(node, "image").map(|i| attr(&i, "path")).unwrap_or_default(),
        files: kid(node, "files").map(parse_rules).unwrap_or_default(),
        flags: kid(node, "conditionFlags")
            .map(|f| kids(f, "flag").map(|x| (attr(&x, "name"), text(Some(x)))).collect())
            .unwrap_or_default(),
        default_type: "Optional".into(),
        patterns: Vec::new(),
    };
    if let Some(td) = kid(node, "typeDescriptor") {
        if let Some(t) = kid(td, "type") {
            plugin.default_type = attr(&t, "name");
        } else if let Some(dt) = kid(td, "dependencyType") {
            if let Some(d) = kid(dt, "defaultType") {
                plugin.default_type = attr(&d, "name");
            }
            if let Some(pats) = kid(dt, "patterns") {
                for p in kids(pats, "pattern") {
                    if let (Some(deps), Some(t)) = (kid(p, "dependencies"), kid(p, "type")) {
                        plugin.patterns.push((parse_dep(deps), attr(&t, "name")));
                    }
                }
            }
        }
    }
    plugin
}

pub fn parse(xml: &str) -> Option<Config> {
    let doc = roxmltree::Document::parse_with_options(xml, roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() }).ok()?;
    let root = doc.root_element();
    if !is(&root, "config") {
        return None;
    }
    let mut steps = Vec::new();
    let mut step_order = String::new();
    if let Some(is_node) = kid(root, "installSteps") {
        step_order = attr(&is_node, "order");
        for s in kids(is_node, "installStep") {
            let group_order = kid(s, "optionalFileGroups").map(|g| attr(&g, "order")).unwrap_or_default();
            let groups = kid(s, "optionalFileGroups")
                .map(|g| {
                    kids(g, "group")
                        .map(|g| Group {
                            name: attr(&g, "name"),
                            kind: attr(&g, "type"),
                            order: kid(g, "plugins").map(|p| attr(&p, "order")).unwrap_or_default(),
                            plugins: kid(g, "plugins").map(|p| kids(p, "plugin").map(parse_plugin).collect()).unwrap_or_default(),
                        })
                        .collect()
                })
                .unwrap_or_default();
            steps.push(Step { name: attr(&s, "name"), visible: kid(s, "visible").map(parse_dep), order: group_order, groups });
        }
    }
    let mut patterns = Vec::new();
    if let Some(c) = kid(root, "conditionalFileInstalls") {
        if let Some(pats) = kid(c, "patterns") {
            for p in kids(pats, "pattern") {
                if let Some(deps) = kid(p, "dependencies") {
                    patterns.push(Pattern { dep: parse_dep(deps), files: kid(p, "files").map(parse_rules).unwrap_or_default() });
                }
            }
        }
    }
    Some(Config {
        name: text(kid(root, "moduleName")),
        order: step_order,
        required: kid(root, "requiredInstallFiles").map(parse_rules).unwrap_or_default(),
        steps,
        patterns,
    })
}

impl Config {
    pub fn view(&self) -> View {
        View {
            name: self.name.clone(),
            order: self.order.clone(),
            steps: self
                .steps
                .iter()
                .map(|s| StepView {
                    name: s.name.clone(),
                    order: s.order.clone(),
                    groups: s
                        .groups
                        .iter()
                        .map(|g| GroupView {
                            name: g.name.clone(),
                            kind: g.kind.clone(),
                            order: g.order.clone(),
                            plugins: g.plugins.iter().map(|p| PluginView { name: p.name.clone(), description: p.description.clone(), image: p.image.clone() }).collect(),
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

fn exists_ci(game: &Path, rel: &str) -> bool {
    let mut here = game.to_path_buf();
    for part in rel.replace('\\', "/").split('/').filter(|p| !p.is_empty()) {
        let Ok(entries) = std::fs::read_dir(&here) else { return false };
        let Some(found) = entries.flatten().find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(part)) else { return false };
        here = found.path();
    }
    here.exists()
}

fn holds(dep: &Dep, flags: &HashMap<String, String>, game: &Path) -> bool {
    match dep {
        Dep::Flag(name, value) => flags.get(name).map(String::as_str).unwrap_or("") == value.as_str(),
        Dep::File(file, state) => {
            let there = exists_ci(game, file);
            if state.eq_ignore_ascii_case("missing") || state.eq_ignore_ascii_case("inactive") {
                !there
            } else {
                there
            }
        }
        Dep::Group { and, items } => {
            if items.is_empty() {
                true
            } else if *and {
                items.iter().all(|d| holds(d, flags, game))
            } else {
                items.iter().any(|d| holds(d, flags, game))
            }
        }
    }
}

fn plugin_type(p: &Plugin, flags: &HashMap<String, String>, game: &Path) -> String {
    p.patterns.iter().find(|(d, _)| holds(d, flags, game)).map(|(_, t)| t.clone()).unwrap_or_else(|| p.default_type.clone())
}

pub fn evaluate(cfg: &Config, selection: &Selection, game: &Path) -> Evaluation {
    let mut flags: HashMap<String, String> = HashMap::new();
    let mut visible = Vec::new();
    let mut types = Vec::new();
    for (si, step) in cfg.steps.iter().enumerate() {
        let shown = step.visible.as_ref().map(|d| holds(d, &flags, game)).unwrap_or(true);
        visible.push(shown);
        let mut step_types = Vec::new();
        for (gi, group) in step.groups.iter().enumerate() {
            let mut group_types = Vec::new();
            for (pi, plugin) in group.plugins.iter().enumerate() {
                let kind = plugin_type(plugin, &flags, game);
                let picked = selection.get(si).and_then(|s| s.get(gi)).is_some_and(|g| g.contains(&pi));
                if shown && (picked || kind.eq_ignore_ascii_case("required")) {
                    for (k, v) in &plugin.flags {
                        flags.insert(k.clone(), v.clone());
                    }
                }
                group_types.push(kind);
            }
            step_types.push(group_types);
        }
        types.push(step_types);
    }
    Evaluation { visible, types }
}

fn norm(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches("./").trim_matches('/').to_string()
}

fn apply(rules: &[Rule], entries: &[(usize, String)], order: &mut usize, out: &mut HashMap<String, (usize, String, i32, usize)>) {
    for rule in rules {
        let source = norm(&rule.source);
        let source_lower = source.to_ascii_lowercase();
        let dest = norm(&rule.dest);
        for (idx, entry) in entries {
            let entry_n = norm(entry);
            let lower = entry_n.to_ascii_lowercase();
            let target = if rule.folder {
                let inside = source.is_empty() || lower.starts_with(&format!("{source_lower}/"));
                if !inside {
                    continue;
                }
                let rest = if source.is_empty() { entry_n.clone() } else { entry_n[source.len() + 1..].to_string() };
                if dest.is_empty() { rest } else { format!("{dest}/{rest}") }
            } else {
                if lower != source_lower {
                    continue;
                }
                let name = entry_n.rsplit('/').next().unwrap_or(&entry_n).to_string();
                if rule.dest.is_empty() || rule.dest.replace('\\', "/").ends_with('/') {
                    if dest.is_empty() { name } else { format!("{dest}/{name}") }
                } else {
                    dest.clone()
                }
            };
            *order += 1;
            let key = target.to_ascii_lowercase();
            let better = out.get(&key).is_none_or(|(_, _, p, o)| rule.priority > *p || (rule.priority == *p && *order > *o));
            if better {
                out.insert(key, (*idx, target, rule.priority, *order));
            }
        }
    }
}

pub fn resolve(cfg: &Config, selection: &Selection, entries: &[(usize, String)], game: &Path) -> Vec<(usize, String)> {
    let eval = evaluate(cfg, selection, game);
    let mut flags: HashMap<String, String> = HashMap::new();
    let mut out: HashMap<String, (usize, String, i32, usize)> = HashMap::new();
    let mut order = 0usize;
    apply(&cfg.required, entries, &mut order, &mut out);
    for (si, step) in cfg.steps.iter().enumerate() {
        if !eval.visible[si] {
            continue;
        }
        for (gi, group) in step.groups.iter().enumerate() {
            for (pi, plugin) in group.plugins.iter().enumerate() {
                let kind = &eval.types[si][gi][pi];
                let picked = selection.get(si).and_then(|s| s.get(gi)).is_some_and(|g| g.contains(&pi));
                if picked || kind.eq_ignore_ascii_case("required") {
                    apply(&plugin.files, entries, &mut order, &mut out);
                    for (k, v) in &plugin.flags {
                        flags.insert(k.clone(), v.clone());
                    }
                }
            }
        }
    }
    for pattern in &cfg.patterns {
        if holds(&pattern.dep, &flags, game) {
            apply(&pattern.files, entries, &mut order, &mut out);
        }
    }
    let mut files: Vec<(usize, String, usize)> = out.into_values().map(|(i, t, _, o)| (i, t, o)).collect();
    files.sort_by_key(|f| f.2);
    files.into_iter().map(|(i, t, _)| (i, t)).collect()
}

pub fn config_path(entries: &[String]) -> Option<(usize, String)> {
    entries.iter().enumerate().find_map(|(i, n)| {
        let norm = n.replace('\\', "/");
        let lower = norm.to_ascii_lowercase();
        lower.strip_suffix("fomod/moduleconfig.xml").and_then(|base| (base.is_empty() || base.ends_with('/')).then(|| (i, norm[..base.len()].to_string())))
    })
}

pub fn build_zip(work: &Path, files: &[(usize, String)], out: &Path) -> Result<(), crate::error::AppError> {
    use std::io::Write;
    let mut archive = zip::ZipArchive::new(std::fs::File::open(work)?)?;
    let mut writer = zip::ZipWriter::new(std::fs::File::create(out)?);
    let opts = zip::write::SimpleFileOptions::default();
    for (idx, dest) in files {
        let mut entry = archive.by_index(*idx)?;
        writer.start_file(dest.as_str(), opts)?;
        std::io::copy(&mut entry, &mut writer)?;
    }
    writer.flush()?;
    writer.finish()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const XML: &str = r#"<config>
  <moduleName>Fancy Outfit</moduleName>
  <requiredInstallFiles><folder source="Core" destination="archive\pc\mod" priority="0"/></requiredInstallFiles>
  <installSteps order="Explicit">
    <installStep name="Style">
      <optionalFileGroups order="Explicit">
        <group name="Color" type="SelectExactlyOne">
          <plugins order="Explicit">
            <plugin name="Red"><description>Red one</description><image path="images\red.png"/>
              <conditionFlags><flag name="color">red</flag></conditionFlags>
              <files><file source="Variants\red.archive" destination="archive\pc\mod\red.archive"/></files>
              <typeDescriptor><type name="Recommended"/></typeDescriptor></plugin>
            <plugin name="Blue">
              <conditionFlags><flag name="color">blue</flag></conditionFlags>
              <files><file source="Variants\blue.archive" destination="archive\pc\mod\"/></files>
              <typeDescriptor><type name="Optional"/></typeDescriptor></plugin>
          </plugins>
        </group>
      </optionalFileGroups>
    </installStep>
    <installStep name="Extras">
      <visible><flagDependency flag="color" value="blue"/></visible>
      <optionalFileGroups><group name="Bits" type="SelectAny"><plugins>
        <plugin name="Hat"><files><file source="hat.xl"/></files>
          <typeDescriptor><dependencyType><defaultType name="Optional"/><patterns><pattern>
            <dependencies operator="Or"><flagDependency flag="color" value="blue"/></dependencies><type name="Recommended"/></pattern></patterns></dependencyType></typeDescriptor></plugin>
      </plugins></group></optionalFileGroups>
    </installStep>
  </installSteps>
  <conditionalFileInstalls><patterns><pattern>
    <dependencies><flagDependency flag="color" value="red"/></dependencies>
    <files><file source="redfx.reds" destination="r6\scripts\redfx.reds" priority="2"/></files>
  </pattern></patterns></conditionalFileInstalls>
</config>"#;

    fn entries() -> Vec<(usize, String)> {
        ["Core/a.archive", "Core/sub/b.archive", "Variants/red.archive", "Variants/blue.archive", "hat.xl", "redfx.reds", "fomod/ModuleConfig.xml"]
            .iter()
            .enumerate()
            .map(|(i, n)| (i, n.to_string()))
            .collect()
    }

    fn names(out: Vec<(usize, String)>) -> Vec<String> {
        let mut v: Vec<String> = out.into_iter().map(|(_, n)| n).collect();
        v.sort();
        v
    }

    #[test]
    fn a_config_is_read_into_steps_groups_and_plugins() {
        let cfg = parse(XML).unwrap();
        let view = cfg.view();
        assert_eq!(view.name, "Fancy Outfit");
        assert_eq!(view.steps.len(), 2);
        assert_eq!(view.steps[0].groups[0].kind, "SelectExactlyOne");
        assert_eq!(view.steps[0].groups[0].plugins[0].description, "Red one");
        assert_eq!(view.steps[0].groups[0].plugins[0].image, "images\\red.png");
        let xml = r#"<config><installSteps order="Ascending"><installStep name="B"><optionalFileGroups order="Descending"><group name="G" type="SelectAny"><plugins order="Explicit"/></group></optionalFileGroups></installStep></installSteps></config>"#;
        let ordered = parse(xml).unwrap().view();
        assert_eq!((ordered.order.as_str(), ordered.steps[0].order.as_str(), ordered.steps[0].groups[0].order.as_str()), ("Ascending", "Descending", "Explicit"));
    }

    #[test]
    fn the_chosen_variant_and_its_flag_driven_extras_decide_the_files() {
        let cfg = parse(XML).unwrap();
        let game = Path::new("Z:/none");
        let red = names(resolve(&cfg, &vec![vec![vec![0]], vec![vec![]]], &entries(), game));
        assert_eq!(red, vec!["archive/pc/mod/red.archive", "archive/pc/mod/sub/b.archive", "archive/pc/mod/a.archive", "r6/scripts/redfx.reds"].into_iter().map(String::from).collect::<Vec<_>>().tap_sorted());
        let blue = names(resolve(&cfg, &vec![vec![vec![1]], vec![vec![0]]], &entries(), game));
        assert_eq!(blue, vec!["archive/pc/mod/a.archive", "archive/pc/mod/blue.archive", "archive/pc/mod/sub/b.archive", "hat.xl"].into_iter().map(String::from).collect::<Vec<_>>());
    }

    #[test]
    fn steps_follow_flags_and_types_follow_dependency_patterns() {
        let cfg = parse(XML).unwrap();
        let game = Path::new("Z:/none");
        let red = evaluate(&cfg, &vec![vec![vec![0]], vec![vec![]]], game);
        assert_eq!(red.visible, vec![true, false]);
        let blue = evaluate(&cfg, &vec![vec![vec![1]], vec![vec![]]], game);
        assert_eq!(blue.visible, vec![true, true]);
        assert_eq!(blue.types[1][0][0], "Recommended");
        assert_eq!(red.types[0][0][0], "Recommended");
    }

    #[test]
    fn higher_priority_wins_for_the_same_destination() {
        let xml = r#"<config><requiredInstallFiles>
            <file source="a.archive" destination="x/t.archive" priority="1"/>
            <file source="b.archive" destination="x/t.archive" priority="5"/>
            <file source="c.archive" destination="x/t.archive" priority="5"/></requiredInstallFiles></config>"#;
        let cfg = parse(xml).unwrap();
        let entries: Vec<(usize, String)> = ["a.archive", "b.archive", "c.archive"].iter().enumerate().map(|(i, n)| (i, n.to_string())).collect();
        assert_eq!(resolve(&cfg, &vec![], &entries, Path::new("Z:/none")), vec![(2, "x/t.archive".to_string())]);
    }

    #[test]
    fn utf16_and_bom_encoded_configs_are_decoded() {
        let text = "<config><moduleName>Ünï</moduleName></config>";
        let mut le = vec![0xFF, 0xFE];
        le.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
        assert_eq!(parse(&decode(&le)).unwrap().name, "Ünï");
        let mut utf8 = vec![0xEF, 0xBB, 0xBF];
        utf8.extend(text.as_bytes());
        assert_eq!(parse(&decode(&utf8)).unwrap().name, "Ünï");
        assert!(parse("<nope/>").is_none());
        assert!(parse("not xml at all").is_none());
    }

    #[test]
    fn the_config_is_found_under_a_wrapper_folder_and_the_zip_is_built_from_the_choice() {
        use std::io::{Read, Write};
        let list = vec!["Mod v1/readme.txt".to_string(), "Mod v1/Fomod/moduleconfig.xml".to_string()];
        assert_eq!(config_path(&list), Some((1, "Mod v1/".to_string())));
        assert_eq!(config_path(&["fomod/ModuleConfig.xml".to_string()]), Some((0, String::new())));
        assert_eq!(config_path(&["notfomod/ModuleConfig.xml".to_string()]), None);
        let dir = tempfile::tempdir().unwrap();
        let work = dir.path().join("w.zip");
        {
            let mut w = zip::ZipWriter::new(std::fs::File::create(&work).unwrap());
            for (n, body) in [("a.archive", "AAA"), ("b.archive", "BBB")] {
                w.start_file(n, zip::write::SimpleFileOptions::default()).unwrap();
                w.write_all(body.as_bytes()).unwrap();
            }
            w.finish().unwrap();
        }
        let out = dir.path().join("o.zip");
        build_zip(&work, &[(1, "archive/pc/mod/b.archive".to_string())], &out).unwrap();
        let mut z = zip::ZipArchive::new(std::fs::File::open(&out).unwrap()).unwrap();
        assert_eq!(z.len(), 1);
        let mut body = String::new();
        z.by_name("archive/pc/mod/b.archive").unwrap().read_to_string(&mut body).unwrap();
        assert_eq!(body, "BBB");
    }

    trait Sorted {
        fn tap_sorted(self) -> Self;
    }
    impl Sorted for Vec<String> {
        fn tap_sorted(mut self) -> Self {
            self.sort();
            self
        }
    }
}
