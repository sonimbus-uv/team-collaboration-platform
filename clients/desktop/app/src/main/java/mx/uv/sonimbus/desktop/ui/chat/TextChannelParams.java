package mx.uv.sonimbus.desktop.ui.chat;

import mx.uv.sonimbus.desktop.data.core.Channel;
import mx.uv.sonimbus.desktop.data.core.Group;

/** Canal de texto que abre el shell, con su grupo. */
public record TextChannelParams(Group group, Channel channel) {
}
