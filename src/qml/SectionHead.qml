import QtQuick

Item {
    property string leftText: ""
    property string rightText: ""
    implicitHeight: 15
    Theme { id: th }
    Label2 { text: leftText; px: 7.5; ls: 0.20; color: th.dimmer }
    Label2 { anchors.right: parent.right; text: rightText; px: 7; color: th.faint }
}
