mod workers;
pub use workers::Worker;

mod sequential_bus;
pub use sequential_bus::MessageBusSeq;

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn it_works() {
//         let result = add(2, 2);
//         assert_eq!(result, 4);
//     }
// }
