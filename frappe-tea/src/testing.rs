api_planning! {
    /// How will we test cmds?
    /// Cmds will have the type
    type Cmd = Box<dyn Cmd>;

    struct BatchCmd(/* ... */);

    struct MyCmd(/* ... */);

    let cmd = MyCmd::new().into_cmd();

    assert_eq!(cmd, MyCmd::new().into_cmd());

    let cmds = BatchCmd::from([cmd.clone().into_cmd(), cmd.clone().into_cmd()]);

    assert_eq!(cmds, [cmd.clone().into_cmd(), cmd.clone().into_cmd()]);
}
