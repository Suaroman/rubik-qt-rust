import QtQuick

// A rail's chrome: 1px rule, then "LEFT / RIGHT" caption with a dim note on the right.
Item {
    property string leftText: ""
    property string rightText: ""
    property string note: ""
    implicitHeight: 30
    Theme { id: th }
    Rectangle { width: parent.width; height: 1; color: th.rule }
    Row {
        y: 13
        Label2 { text: leftText; px: 8; ls: 0.22; color: th.dimmer }
        Label2 { text: " / "; px: 8; ls: 0.22; color: "#454d60" }
        Label2 { text: rightText; px: 8; ls: 0.22; color: th.dimmer }
    }
    Label2 { y: 14; anchors.right: parent.right; text: note; px: 7.5; ls: 0.14; color: "#39404f" }
}
