type Node = any;
type Component<OptionalProps extends Object, RequiredProps extends object> = {
    _internal: (key: string, props: Partial<OptionalProps> & RequiredProps) => Node
};

declare function define_component<OptionalProps extends object, RequiredProps extends object>(
    fun: (props: OptionalProps & RequiredProps) => Node,
    optional_prop_defaults: OptionalProps
): Component<OptionalProps, RequiredProps>;

declare function constructComponent<OptionalProps extends object, RequiredProps extends object>(
    component: Component<OptionalProps, RequiredProps>,
    key: string,
    props: Partial<OptionalProps> & RequiredProps,
)
