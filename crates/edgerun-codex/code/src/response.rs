use edgerun_json::{FromJson, JsonValueError, Map, ToJson, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageDetail {
    Auto,
    Low,
    High,
    Original,
}

impl ToJson for ImageDetail {
    fn to_json(&self) -> Value {
        Value::String(
            match self {
                Self::Auto => "auto",
                Self::Low => "low",
                Self::High => "high",
                Self::Original => "original",
            }
            .to_string(),
        )
    }
}

impl FromJson for ImageDetail {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let value = String::from_json(value)?;
        match value.as_str() {
            "auto" => Ok(Self::Auto),
            "low" => Ok(Self::Low),
            "high" => Ok(Self::High),
            "original" => Ok(Self::Original),
            _ => Err(JsonValueError::WrongType(format!(
                "unknown image detail `{value}`"
            ))),
        }
    }
}

pub const DEFAULT_IMAGE_DETAIL: ImageDetail = ImageDetail::High;

#[derive(Debug, Clone, PartialEq)]
pub enum FunctionCallOutputContentItem {
    InputText {
        text: String,
    },
    InputImage {
        image_url: String,
        detail: Option<ImageDetail>,
    },
}

impl ToJson for FunctionCallOutputContentItem {
    fn to_json(&self) -> Value {
        let mut object = Map::new();
        match self {
            Self::InputText { text } => {
                object.push_field("type", "input_text");
                object.push_field("text", text.as_str());
            }
            Self::InputImage { image_url, detail } => {
                object.push_field("type", "input_image");
                object.push_field("image_url", image_url.as_str());
                object.push_opt_field("detail", detail.as_ref().map(ToJson::to_json));
            }
        }
        object.into()
    }
}

impl FromJson for FunctionCallOutputContentItem {
    fn from_json(value: Value) -> Result<Self, JsonValueError> {
        let mut object = value.into_object("FunctionCallOutputContentItem")?;
        let item_type: String = object.take_required("type")?;
        match item_type.as_str() {
            "input_text" => Ok(Self::InputText {
                text: object.take_required("text")?,
            }),
            "input_image" => Ok(Self::InputImage {
                image_url: object.take_required("image_url")?,
                detail: object.take_optional("detail")?,
            }),
            _ => Err(JsonValueError::WrongType(format!(
                "unknown function call output content item type `{item_type}`"
            ))),
        }
    }
}
