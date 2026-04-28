use edgerun_json::{from_json_value, impl_json_struct, json, to_json_value, FromJson, ToJson};

#[derive(Debug, PartialEq, Eq)]
struct Job {
    id: String,
    priority: u32,
    tags: Vec<String>,
    note: Option<String>,
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
        }
    }
}

#[test]
fn model_macro_expands_for_external_users_without_serde() {
    let job = Job {
        id: String::from("job-1"),
        priority: 7,
        tags: vec![String::from("edge"), String::from("no-std")],
        note: None,
    };

    let value = to_json_value(&job);
    assert_eq!(value.required_str("id").unwrap(), "job-1");
    assert_eq!(value.required_u32("priority").unwrap(), 7);
    assert!(value.get("note").is_none());

    let decoded = from_json_value::<Job>(json!({
        "id": "job-2",
        "priority": 3,
        "tags": ["bare"],
        "note": "ready"
    }))
    .unwrap();

    assert_eq!(
        decoded,
        Job {
            id: String::from("job-2"),
            priority: 3,
            tags: vec![String::from("bare")],
            note: Some(String::from("ready")),
        }
    );
}
