use super::*;

use collomatique_rpc::CompleteCmdMsg;
use collomatique_state_colloscopes::Data;

/// The default document with one export colour changed
///
/// Two such documents with different `red` serialize to the same length, so a
/// token reading anything less than the content itself would confuse them.
fn recoloured_data(red: u8) -> Data {
    let mut inner_data = Data::default().get_inner_data().clone();
    inner_data.export_config.global.background_color.red = red;
    Data::from_inner_data(inner_data).expect("recolouring keeps the document valid")
}

#[test]
fn token_depends_on_the_document_and_on_nothing_else() {
    let stream = InternalDataStream::from(&recoloured_data(100));

    assert_eq!(stream.token(), stream.token());
    assert_eq!(
        stream.token(),
        InternalDataStream::from(&recoloured_data(100)).token(),
        "two runs of the writer on the same document agree"
    );
    assert_ne!(
        stream.token(),
        InternalDataStream::from(&recoloured_data(101)).token(),
        "an edited document is a different document"
    );
}

#[test]
fn console_messages_round_trip() {
    let init = ColloInitMsg::App(AppInitMsg::StartPythonRepl);
    assert_eq!(
        ColloInitMsg::from_text_msg(&init.to_text_msg()).unwrap(),
        init
    );

    for cmd in [
        AppCmdMsg::ReadLine {
            prompt: ">>> ".to_string(),
        },
        AppCmdMsg::ReplaceData {
            data: InternalDataStream::from(&Data::default()),
            token: Some(42),
        },
        AppCmdMsg::ReplaceData {
            data: InternalDataStream::from(&recoloured_data(101)),
            token: None,
        },
    ] {
        let msg = CompleteCmdMsg::CmdMsg(ColloCmdMsg::App(cmd));
        assert_eq!(
            CompleteCmdMsg::from_text_msg(&msg.to_text_msg()).unwrap(),
            msg
        );
    }

    for answer in [
        AppAnswerMsg::Line("1 + 1".to_string()),
        AppAnswerMsg::ReplaceDone { token: 42 },
        AppAnswerMsg::ReplaceRefused,
    ] {
        let msg = ColloResultMsg::App(answer);
        assert_eq!(
            ColloResultMsg::from_text_msg(&msg.to_text_msg()).unwrap(),
            msg
        );
    }
}
