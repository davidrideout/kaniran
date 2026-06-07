mod optprefix {
    use crate::dict::split::split::*;

    #[test]
    fn already_has_prefix() {
        assert_eq!(optprefix("い")("いう"), "いう");
    }

    #[test]
    fn missing_prefix_prepended() {
        assert_eq!(optprefix("い")("う"), "いう");
    }

    #[test]
    fn empty_input_takes_prefix() {
        assert_eq!(optprefix("い")(""), "い");
    }
}
