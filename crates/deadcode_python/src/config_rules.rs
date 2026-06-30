use super::{ConfigError, RuleConfig};

pub(super) fn validate_rules(rules: &RuleConfig) -> Result<(), ConfigError> {
    validate_constructors(rules)?;
    validate_factory_returns(rules)?;
    validate_class_surfaces(rules)?;
    validate_decorators(rules)?;
    validate_calls(rules)?;
    validate_fluent_methods(rules)?;
    validate_route_globs(rules)?;
    validate_assignments(rules)?;
    Ok(())
}

fn validate_constructors(rules: &RuleConfig) -> Result<(), ConfigError> {
    for constructor in &rules.constructors {
        if constructor.match_.trim().is_empty() {
            return invalid_rule("constructor match must not be empty");
        }
        if constructor.produces_type.trim().is_empty() {
            return invalid_rule("constructor producesType must not be empty");
        }
    }
    Ok(())
}

fn validate_factory_returns(rules: &RuleConfig) -> Result<(), ConfigError> {
    for factory_return in &rules.factory_returns {
        if factory_return.function.trim().is_empty() {
            return invalid_rule("factory return function must not be empty");
        }
        if factory_return.type_keyword.trim().is_empty() && factory_return.type_position.is_none() {
            return invalid_rule("factory return typeKeyword or typePosition must be configured");
        }
        if factory_return
            .input_type_keyword
            .as_ref()
            .is_some_and(|keyword| keyword.trim().is_empty())
        {
            return invalid_rule("factory return inputTypeKeyword must not be empty");
        }
        if let Some(container) = &factory_return.return_container {
            if container != "list" {
                return invalid_rule(format!("unsupported factory returnContainer {container}"));
            }
        }
    }
    Ok(())
}

fn validate_class_surfaces(rules: &RuleConfig) -> Result<(), ConfigError> {
    for class_surface in &rules.class_surfaces {
        if class_surface.base.trim().is_empty() {
            return invalid_rule("class surface base must not be empty");
        }
        if class_surface.effect != "markClassAttributes" {
            return invalid_rule(format!(
                "unsupported class surface effect {}",
                class_surface.effect
            ));
        }
    }
    Ok(())
}

fn validate_decorators(rules: &RuleConfig) -> Result<(), ConfigError> {
    for decorator in &rules.decorators {
        if decorator.function.is_none() && decorator.receiver_type.is_none() {
            return invalid_rule("decorator rules require function or receiverType plus methods");
        }
        if decorator
            .function
            .as_ref()
            .is_some_and(|function| function.trim().is_empty())
        {
            return invalid_rule("decorator function must not be empty");
        }
        if decorator
            .receiver_type
            .as_ref()
            .is_some_and(|receiver_type| receiver_type.trim().is_empty())
        {
            return invalid_rule("decorator receiverType must not be empty");
        }
        if decorator.receiver_type.is_some() && decorator.methods.is_empty() {
            return invalid_rule("decorator receiverType rules require methods");
        }
        if !matches!(
            decorator.effect.as_str(),
            "registerDecoratedFunction"
                | "registerBoundaryFunction"
                | "wrapWithCallableType"
                | "useFunctionParameters"
        ) {
            return invalid_rule(format!("unsupported decorator effect {}", decorator.effect));
        }
        if decorator.effect == "wrapWithCallableType"
            && decorator
                .callable_type
                .as_ref()
                .is_none_or(|callable_type| callable_type.trim().is_empty())
        {
            return invalid_rule("wrapWithCallableType decorator rules require callableType");
        }
    }
    Ok(())
}

fn validate_calls(rules: &RuleConfig) -> Result<(), ConfigError> {
    for call in &rules.calls {
        if call.function.is_none() && (call.receiver_type.is_none() || call.method.is_none()) {
            return invalid_rule("call rules require function or receiverType plus method");
        }
        if !matches!(
            call.effect.as_str(),
            "useCallableArgument" | "connectRouter" | "useArgumentMember" | "replaceCallableReturn"
        ) {
            return invalid_rule(format!("unsupported call effect {}", call.effect));
        }
        if call.effect == "useArgumentMember"
            && call
                .member
                .as_ref()
                .is_none_or(|member| member.trim().is_empty())
        {
            return invalid_rule("useArgumentMember call rules require member");
        }
    }
    Ok(())
}

fn validate_fluent_methods(rules: &RuleConfig) -> Result<(), ConfigError> {
    for fluent_method in &rules.fluent_methods {
        if fluent_method.receiver_type.trim().is_empty() {
            return invalid_rule("fluent method receiverType must not be empty");
        }
        if fluent_method.methods.is_empty() {
            return invalid_rule("fluent method methods must not be empty");
        }
    }
    Ok(())
}

fn validate_route_globs(rules: &RuleConfig) -> Result<(), ConfigError> {
    for route_glob in &rules.route_globs {
        if route_glob.when_function_called.trim().is_empty() {
            return invalid_rule("route glob whenFunctionCalled must not be empty");
        }
        if route_glob.glob.trim().is_empty() {
            return invalid_rule("route glob glob must not be empty");
        }
        if route_glob.export.trim().is_empty() {
            return invalid_rule("route glob export must not be empty");
        }
        if route_glob.effect != "includeRouter" {
            return invalid_rule(format!(
                "unsupported route glob effect {}",
                route_glob.effect
            ));
        }
    }
    Ok(())
}

fn validate_assignments(rules: &RuleConfig) -> Result<(), ConfigError> {
    for assignment in &rules.assignments {
        if !matches!(assignment.effect.as_str(), "overrideCallableReturn") {
            return invalid_rule(format!(
                "unsupported assignment effect {}",
                assignment.effect
            ));
        }
        if assignment.receiver_type.trim().is_empty() || assignment.member.trim().is_empty() {
            return invalid_rule("assignment rules require receiverType and member");
        }
    }
    Ok(())
}

fn invalid_rule<T>(message: impl Into<String>) -> Result<T, ConfigError> {
    Err(ConfigError::InvalidRule {
        message: message.into(),
    })
}
