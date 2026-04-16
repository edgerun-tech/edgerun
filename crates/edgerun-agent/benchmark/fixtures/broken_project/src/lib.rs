// This project has intentional issues that cargo check will report
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

// Deliberate bug: unused variable
pub fn broken_function() {
    let unused = 42;
    println!("broken");
}

// Missing return type (type annotation required)
pub fn needs_return_type(x) -> String {
    format!("{:?}", x)
}
