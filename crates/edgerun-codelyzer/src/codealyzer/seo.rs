use crate::codealyzer::crate_model::CrateReport;

pub fn generate_meta_tags(report: &CrateReport) -> String {
    let mut tags = String::new();

    let title = format!("{} - Crate Analysis Report", report.identity.name);
    let description = format!(
        "EdgeRun crate analysis for {} v{}",
        report.identity.name, report.identity.version
    );

    tags.push_str(&format!("<title>{}</title>\n", title));
    tags.push_str(&format!(
        "<meta name=\"description\" content=\"{}\">\n",
        description
    ));

    tags.push_str(&format!(
        "<meta property=\"og:title\" content=\"{}\">\n",
        title
    ));
    tags.push_str(&format!(
        "<meta property=\"og:description\" content=\"{}\">\n",
        description
    ));
    tags.push_str("<meta property=\"og:type\" content=\"article\">\n");

    tags.push_str(&format!(
        "<meta name=\"twitter:title\" content=\"{}\">\n",
        title
    ));
    tags.push_str(&format!(
        "<meta name=\"twitter:description\" content=\"{}\">\n",
        description
    ));

    tags
}

pub fn generate_json_ld(report: &CrateReport) -> String {
    format!(
        "{{\
        \"@context\": \"https://schema.org\",\
        \"@type\": \"SoftwareSourceCode\",\
        \"name\": \"{}\",\
        \"version\": \"{}\",\
        \"description\": \"{}\",\
        \"programmingLanguage\": \"Rust\",\
        \"dateCreated\": \"{}\"\
    }}",
        report.identity.name,
        report.identity.version,
        report.identity.description.as_deref().unwrap_or(""),
        report.generated_at
    )
}
