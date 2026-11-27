import 'package:flutter/material.dart';

import '../theme/app-theme-extension.dart';

class TextChannelView extends StatefulWidget {
  final String channelName;
  final String channelDescription;

  const TextChannelView({
    super.key,
    this.channelName = 'general',
    this.channelDescription = 'Avisos y conversación general del grupo',
  });

  @override
  State<TextChannelView> createState() => _TextChannelViewState();
}

class _TextChannelViewState extends State<TextChannelView> {
  final TextEditingController _messageController = TextEditingController();
  final List<ChatMessage> _messages = [
    ChatMessage(
      author: 'Ana López',
      time: '10:00',
      text: '¿Ya revisaron el documento de especificación? Faltan los diagramas de secuencia.',
      reactions: const ['De acuerdo (3)', 'Visto (1)'],
    ),
    ChatMessage(
      author: 'Luis Vázquez (tú)',
      time: '10:05',
      text: 'Sí, ya agregué la sección de casos de uso. Los diagramas los subo hoy.',
      isSelf: true,
      replyTo: 'Ana López: ¿Ya revisaron el documento...?',
    ),
    ChatMessage(
      author: 'Carlos Ruiz',
      time: '10:11',
      text: 'Perfecto, lo reviso en la tarde y dejo comentarios.',
    ),
  ];

  bool _isReplying = false;
  String? _replyingToAuthor;
  String? _replyingToText;

  @override
  void dispose() {
    _messageController.dispose();
    super.dispose();
  }

  void _sendMessage() {
    final trimmed = _messageController.text.trim();
    if (trimmed.isEmpty) return;

    setState(() {
      _messages.add(
        ChatMessage(
          author: 'Luis Vázquez (tú)',
          time: TimeOfDay.now().format(context),
          text: trimmed,
          isSelf: true,
          replyTo: _isReplying ? '$_replyingToAuthor: $_replyingToText' : null,
        ),
      );
      _messageController.clear();
      _isReplying = false;
      _replyingToAuthor = null;
      _replyingToText = null;
    });
  }

  void _cancelReply() {
    setState(() {
      _isReplying = false;
      _replyingToAuthor = null;
      _replyingToText = null;
    });
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final themeColors = theme.extension<AppThemeColors>()!;

    return Scaffold(
      backgroundColor: colorScheme.surface,
      appBar: AppBar(
        backgroundColor: colorScheme.surface,
        surfaceTintColor: Colors.transparent,
        elevation: 0,
        bottom: PreferredSize(
          preferredSize: const Size.fromHeight(1),
          child: Container(height: 1, color: themeColors.divider),
        ),
        title: Row(
          children: [
            Text(
              '#${widget.channelName}',
              style: TextStyle(
                fontWeight: FontWeight.bold,
                color: colorScheme.onSurface,
              ),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                widget.channelDescription,
                style: TextStyle(
                  fontSize: 12,
                  color: colorScheme.onSurfaceVariant,
                ),
                overflow: TextOverflow.ellipsis,
              ),
            ),
          ],
        ),
        iconTheme: IconThemeData(color: colorScheme.onSurface),
      ),
      body: Column(
        children: [
          // ================= Historial de mensajes =================
          Expanded(
            child: _messages.isEmpty
                ? Center(
                    child: Text(
                      'Todavía no hay mensajes en este canal',
                      style: TextStyle(
                        color: colorScheme.onSurfaceVariant,
                        fontSize: 14,
                      ),
                    ),
                  )
                : ListView.builder(
                    padding: const EdgeInsets.all(16),
                    itemCount: _messages.length,
                    itemBuilder: (context, index) {
                      final msg = _messages[index];
                      return MessageBubble(
                        message: msg,
                        onReply: () {
                          setState(() {
                            _isReplying = true;
                            _replyingToAuthor = msg.author;
                            _replyingToText = msg.text;
                          });
                        },
                      );
                    },
                  ),
          ),

          // ================= Caja de escritura (Composer) =================
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
            decoration: BoxDecoration(
              color: colorScheme.surface,
              border: Border(top: BorderSide(color: themeColors.divider)),
            ),
            child: SafeArea(
              top: false,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  // Barra de respuesta (CU-17)
                  if (_isReplying)
                    Container(
                      margin: const EdgeInsets.only(bottom: 8),
                      padding: const EdgeInsets.symmetric(
                        horizontal: 12,
                        vertical: 6,
                      ),
                      decoration: BoxDecoration(
                        color: themeColors.sidePanel,
                        borderRadius: BorderRadius.circular(6),
                        border: Border(
                          left: BorderSide(
                            color: colorScheme.primary,
                            width: 3,
                          ),
                        ),
                      ),
                      child: Row(
                        children: [
                          Text(
                            'Respondiendo a ',
                            style: TextStyle(
                              fontSize: 12,
                              color: colorScheme.onSurfaceVariant,
                            ),
                          ),
                          Text(
                            _replyingToAuthor ?? '',
                            style: TextStyle(
                              fontSize: 12,
                              fontWeight: FontWeight.bold,
                              color: colorScheme.onSurface,
                            ),
                          ),
                          const SizedBox(width: 8),
                          Expanded(
                            child: Text(
                              _replyingToText ?? '',
                              style: TextStyle(
                                fontSize: 12,
                                color: colorScheme.onSurfaceVariant,
                              ),
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                          InkWell(
                            onTap: _cancelReply,
                            borderRadius: BorderRadius.circular(4),
                            child: Padding(
                              padding: const EdgeInsets.all(2),
                              child: Icon(
                                Icons.close,
                                size: 16,
                                color: colorScheme.onSurfaceVariant,
                              ),
                            ),
                          ),
                        ],
                      ),
                    ),

                  // Label descriptivo para accesibilidad (WCAG 2.2)
                  Padding(
                    padding: const EdgeInsets.only(bottom: 4),
                    child: Text(
                      'Mensaje para el canal',
                      style: TextStyle(
                        fontSize: 11,
                        fontWeight: FontWeight.w600,
                        color: colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ),

                  // Fila de entrada de texto y botones
                  Row(
                    children: [
                      // Botón Adjuntar (CU-23 / RF-MSG-04)
                      OutlinedButton.icon(
                        onPressed: () {
                          // Lógica para adjuntar archivos o imágenes
                        },
                        icon: const Icon(Icons.attach_file, size: 18),
                        label: const Text('Adjuntar'),
                        style: OutlinedButton.styleFrom(
                          foregroundColor: colorScheme.onSurface,
                          side: BorderSide(color: colorScheme.outlineVariant),
                          padding: const EdgeInsets.symmetric(
                            horizontal: 12,
                            vertical: 12,
                          ),
                          shape: RoundedRectangleBorder(
                            borderRadius: BorderRadius.circular(8),
                          ),
                        ),
                      ),
                      const SizedBox(width: 8),

                      // Campo de texto
                      Expanded(
                        child: TextField(
                          controller: _messageController,
                          style: TextStyle(
                            color: colorScheme.onSurface,
                            fontSize: 14,
                          ),
                          decoration: InputDecoration(
                            hintText: 'Escribe un mensaje…',
                            hintStyle: TextStyle(
                              color: colorScheme.onSurfaceVariant,
                              fontSize: 14,
                            ),
                            filled: true,
                            fillColor: themeColors.sidePanel,
                            contentPadding: const EdgeInsets.symmetric(
                              horizontal: 14,
                              vertical: 10,
                            ),
                            border: OutlineInputBorder(
                              borderRadius: BorderRadius.circular(8),
                              borderSide: BorderSide(
                                color: colorScheme.outlineVariant,
                              ),
                            ),
                            enabledBorder: OutlineInputBorder(
                              borderRadius: BorderRadius.circular(8),
                              borderSide: BorderSide(
                                color: colorScheme.outlineVariant,
                              ),
                            ),
                            focusedBorder: OutlineInputBorder(
                              borderRadius: BorderRadius.circular(8),
                              borderSide: BorderSide(
                                color: colorScheme.primary,
                                width: 1.5,
                              ),
                            ),
                          ),
                          onSubmitted: (_) => _sendMessage(),
                        ),
                      ),
                      const SizedBox(width: 8),

                      // Botón Enviar (CU-16 / RF-MSG-01)
                      ElevatedButton(
                        onPressed: _sendMessage,
                        style: ElevatedButton.styleFrom(
                          backgroundColor: colorScheme.primary,
                          foregroundColor: colorScheme.onPrimary,
                          padding: const EdgeInsets.symmetric(
                            horizontal: 16,
                            vertical: 12,
                          ),
                          shape: RoundedRectangleBorder(
                            borderRadius: BorderRadius.circular(8),
                          ),
                          elevation: 0,
                        ),
                        child: const Text('Enviar'),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}

// Modelo auxiliar para representar un mensaje
class ChatMessage {
  final String author;
  final String time;
  final String text;
  final bool isSelf;
  final String? replyTo;
  final List<String> reactions;

  ChatMessage({
    required this.author,
    required this.time,
    required this.text,
    this.isSelf = false,
    this.replyTo,
    this.reactions = const [],
  });
}

// Componente visual para cada burbuja de mensaje en el historial
class MessageBubble extends StatelessWidget {
  final ChatMessage message;
  final VoidCallback onReply;

  const MessageBubble({
    super.key,
    required this.message,
    required this.onReply,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final themeColors = theme.extension<AppThemeColors>()!;

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6.0),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          CircleAvatar(
            radius: 16,
            backgroundColor: message.isSelf
                ? colorScheme.primaryContainer
                : themeColors.sidePanel,
            child: Text(
              message.author.isNotEmpty ? message.author.substring(0, 1) : '?',
              style: TextStyle(
                fontSize: 12,
                fontWeight: FontWeight.bold,
                color: message.isSelf
                    ? colorScheme.onPrimaryContainer
                    : colorScheme.onSurface,
              ),
            ),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                // Cabecera del mensaje (Autor y hora)
                Row(
                  children: [
                    Text(
                      message.author,
                      style: TextStyle(
                        fontWeight: FontWeight.bold,
                        fontSize: 13,
                        color: message.isSelf
                            ? colorScheme.primary
                            : colorScheme.onSurface,
                      ),
                    ),
                    const SizedBox(width: 8),
                    Text(
                      message.time,
                      style: TextStyle(
                        fontSize: 11,
                        color: colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 2),

                // Referencia si es una respuesta (CU-17)
                if (message.replyTo != null)
                  Container(
                    margin: const EdgeInsets.only(bottom: 4),
                    padding: const EdgeInsets.symmetric(
                      horizontal: 8,
                      vertical: 4,
                    ),
                    decoration: BoxDecoration(
                      color: themeColors.sidePanel,
                      borderRadius: const BorderRadius.only(
                        topRight: Radius.circular(4),
                        bottomRight: Radius.circular(4),
                      ),
                      border: Border(
                        left: BorderSide(color: colorScheme.primary, width: 2),
                      ),
                    ),
                    child: Text(
                      'Respondiendo a ${message.replyTo}',
                      style: TextStyle(
                        fontSize: 11,
                        color: colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ),

                // Texto del mensaje
                Text(
                  message.text,
                  style: TextStyle(fontSize: 13, color: colorScheme.onSurface),
                ),

                // Reacciones asociadas (CU-18)
                if (message.reactions.isNotEmpty)
                  Padding(
                    padding: const EdgeInsets.only(top: 6.0),
                    child: Wrap(
                      spacing: 4,
                      runSpacing: 4,
                      children: message.reactions.map((r) {
                        return Container(
                          padding: const EdgeInsets.symmetric(
                            horizontal: 8,
                            vertical: 2,
                          ),
                          decoration: BoxDecoration(
                            color: themeColors.selectedItem,
                            borderRadius: BorderRadius.circular(10),
                            border: Border.all(color: themeColors.divider),
                          ),
                          child: Text(
                            r,
                            style: TextStyle(
                              fontSize: 10,
                              fontWeight: FontWeight.w500,
                              color: colorScheme.onSurface,
                            ),
                          ),
                        );
                      }).toList(),
                    ),
                  ),
              ],
            ),
          ),
          // Botón de opciones rápidas (Responder)
          IconButton(
            tooltip: 'Responder mensaje',
            icon: Icon(
              Icons.reply,
              size: 16,
              color: colorScheme.onSurfaceVariant,
            ),
            onPressed: onReply,
            constraints: const BoxConstraints(),
            padding: const EdgeInsets.all(4),
          ),
        ],
      ),
    );
  }
}
