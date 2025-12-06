; e.g. `$age = 25` matches `$age`
(expression_statement
    (assignment_expression
        left: (variable_name) @debug-variable
    )
)

; e.g. `++$age` matches `$age`
(expression_statement
    (update_expression
        argument: (variable_name) @debug-variable
    )
)

; e.g. `if ($age > 18)` matches `$age`
(binary_expression
    left: (variable_name) @debug-variable
)

; e.g. `if (18 < $age)` matches `$age`
(binary_expression
    right: (variable_name) @debug-variable
)

; e.g. `__construct(int $age)` matches `$age`
(simple_parameter
    name: (variable_name) @debug-variable
)
