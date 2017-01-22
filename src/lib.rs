#[macro_use]
extern crate error_chain;

mod errors {
    error_chain! {
        errors {
            Deserialize(t: String) {
                description("failed to deserialize")
                display("failed to deserialize: {}",t)
            }
            Serialize(t: String) {
                description("failed to serialize")
                display("failed to serialize: {}",t)
            }
        }
    }
}

use errors::*;
#[cfg(test)]
mod tests {
    use super::*;
}
