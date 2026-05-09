use edgerun_json::{FromJson, ToJson, from_json_value, impl_json_struct, json, to_json_value};

#[derive(Debug, PartialEq, Eq)]
struct Job {
    id: String,
    priority: u32,
    tags: Vec<String>,
    note: Option<String>,
    quota: Option<u64>,
}

impl_json_struct! {
    Job {
        required {
            id: "id" => String,
            priority: "priority" => u32,
            tags: "tags" => Vec<String>,
        }
        optional {
            note: "note" => String,
            quota: ["quota", "legacy_quota"] => u64,
        }
    }
}

#[test]
fn model_macro_expands_for_external_users() {
    let job = Job {
        id: String::from("job-1"),
        priority: 7,
        tags: vec![String::from("edge"), String::from("no-std")],
        note: None,
        quota: Some(42),
    };

    let value = to_json_value(&job);
    assert_eq!(value.required_str("id").unwrap(), "job-1");
    assert_eq!(value.required_u32("priority").unwrap(), 7);
    assert_eq!(value.required_u64("quota").unwrap(), 42);
    assert!(value.get("legacy_quota").is_none());
    assert!(value.get("note").is_none());

    let decoded = from_json_value::<Job>(json!({
        "id": "job-2",
        "priority": 3,
        "tags": ["bare"],
        "note": "ready",
        "legacy_quota": 9
    }))
    .unwrap();

    assert_eq!(
        decoded,
        Job {
            id: String::from("job-2"),
            priority: 3,
            tags: vec![String::from("bare")],
            note: Some(String::from("ready")),
            quota: Some(9),
        }
    );
}
