import 'package:flutter/material.dart';

class TextChannelView extends StatefulWidget {
  final String channelName;
  final String channelDescription;

  const TextChannelView({
    Key? key,
    this.channelName = 'general',
    this.channelDescription = 'Avisos y conversación general del grupo',
  }) : super(key: key);

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

  void _sendMessage() {
    if (_messageController.text.trim().isEmpty) return;
    setState(() {
      _messages.add(
        ChatMessage(
          author: 'Luis Vázquez (tú)',
          time: TimeOfDay.now().format(context),
          text: _messageController.text,
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
    return Scaffold(
      backgroundColor: Colors.white,
      appBar: AppBar(
        backgroundColor: Colors.white,
        elevation: 0.5,
        title: Row(
          children: [
            Text(
              '#${widget.channelName}',
              style: const TextStyle(
                fontWeight: FontWeight.bold,
                color: Color(0xFF1F2937),
              ),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                widget.channelDescription,
                style: const TextStyle(fontSize: 12, color: Color(0xFF6B7280)),
                overflow: TextOverflow.ellipsis,
              ),
            ),
          ],
        ),
        iconTheme: const IconThemeData(color: Color(0xFF374151)),
      ),
      body: Column(
        children: [
          // ================= Historial de mensajes =================
          Expanded(
            child: _messages.isEmpty
                ? const Center(
                    child: Text(
                      'Todavía no hay mensajes en este canal',
                      style: TextStyle(color: Color(0xFF9CA3AF), fontSize: 14),
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
            decoration: const BoxDecoration(
              color: Colors.white,
              border: Border(top: BorderSide(color: Color(0xFFE5E7EB))),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                // Barra de respuesta (CU-17) - Oculta por defecto o según estado
                if (_isReplying)
                  Container(
                    margin: const EdgeInsets.only(bottom: 8),
                    padding: const EdgeInsets.symmetric(
                      horizontal: 12,
                      vertical: 6,
                    ),
                    decoration: BoxDecoration(
                      color: const Color(0xFFF3F4F6),
                      borderRadius: BorderRadius.circular(6),
                      border: const Border(
                        left: BorderSide(color: Color(0xFFC84B10), width: 3),
                      ),
                    ),
                    child: Row(
                      children: [
                        const Text(
                          'Respondiendo a ',
                          style: TextStyle(
                            fontSize: 12,
                            color: Color(0xFF6B7280),
                          ),
                        ),
                        Text(
                          _replyingToAuthor ?? '',
                          style: const TextStyle(
                            fontSize: 12,
                            fontWeight: FontWeight.bold,
                            color: Color(0xFF374151),
                          ),
                        ),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Text(
                            _replyingToText ?? '',
                            style: const TextStyle(
                              fontSize: 12,
                              color: Color(0xFF6B7280),
                            ),
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                        InkWell(
                          onTap: _cancelReply,
                          child: const Icon(
                            Icons.close,
                            size: 16,
                            color: Color(0xFF6B7280),
                          ),
                        ),
                      ],
                    ),
                  ),

                // Label descriptivo para accesibilidad (labelFor equivalente)
                const Padding(
                  padding: EdgeInsets.only(bottom: 4),
                  child: Text(
                    'Mensaje para el canal',
                    style: TextStyle(
                      fontSize: 11,
                      fontWeight: FontWeight.w600,
                      color: Color(0xFF6B7280),
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
                        foregroundColor: const Color(0xFF374151),
                        side: const BorderSide(color: Color(0xFFD1D5DB)),
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
                        decoration: InputDecoration(
                          hintText: 'Escribe un mensaje…',
                          hintStyle: const TextStyle(
                            color: Color(0xFF9CA3AF),
                            fontSize: 14,
                          ),
                          contentPadding: const EdgeInsets.symmetric(
                            horizontal: 14,
                            vertical: 10,
                          ),
                          border: OutlineInputBorder(
                            borderRadius: BorderRadius.circular(8),
                            borderSide: const BorderSide(
                              color: Color(0xFFD1D5DB),
                            ),
                          ),
                          enabledBorder: OutlineInputBorder(
                            borderRadius: BorderRadius.circular(8),
                            borderSide: const BorderSide(
                              color: Color(0xFFD1D5DB),
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
                        backgroundColor: const Color(
                          0xFFC84B10,
                        ), // Color primario corporativo
                        foregroundColor: Colors.white,
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

  const MessageBubble({Key? key, required this.message, required this.onReply})
    : super(key: key);

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6.0),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          CircleAvatar(
            radius: 16,
            backgroundColor: const Color(0xFFE5E7EB),
            child: Text(
              message.author.substring(0, 1),
              style: const TextStyle(
                fontSize: 12,
                fontWeight: FontWeight.bold,
                color: Color(0xFF374151),
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
                      style: const TextStyle(
                        fontWeight: FontWeight.bold,
                        fontSize: 13,
                        color: Color(0xFF1F2937),
                      ),
                    ),
                    const SizedBox(width: 8),
                    Text(
                      message.time,
                      style: const TextStyle(
                        fontSize: 11,
                        color: Color(0xFF9CA3AF),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 2),

                // Referencia si es una respuesta (CU-17)
                if (message.replyTo != null)
                  Container(
                    margin: const EdgeInsets.only(bottom: 4),
                    padding: const EdgeInsets.all(4),
                    decoration: BoxDecoration(
                      color: const Color(0xFFF9FAFB),
                      border: const Border(
                        left: BorderSide(color: Color(0xFF9CA3AF), width: 2),
                      ),
                    ),
                    child: Text(
                      'Respondiendo a ${message.replyTo}',
                      style: const TextStyle(
                        fontSize: 11,
                        color: Color(0xFF6B7280),
                      ),
                    ),
                  ),

                // Texto del mensaje
                Text(
                  message.text,
                  style: const TextStyle(
                    fontSize: 13,
                    color: Color(0xFF374151),
                  ),
                ),

                // Reacciones asociadas (CU-18)
                if (message.reactions.isNotEmpty)
                  Padding(
                    padding: const EdgeInsets.only(top: 4.0),
                    child: Wrap(
                      spacing: 4,
                      children: message.reactions.map((r) {
                        return Container(
                          padding: const EdgeInsets.symmetric(
                            horizontal: 6,
                            vertical: 2,
                          ),
                          decoration: BoxDecoration(
                            color: const Color(0xFFF3F4F6),
                            borderRadius: BorderRadius.circular(10),
                            border: Border.all(color: const Color(0xFFE5E7EB)),
                          ),
                          child: Text(
                            r,
                            style: const TextStyle(
                              fontSize: 10,
                              color: Color(0xFF4B5563),
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
            icon: const Icon(Icons.reply, size: 16, color: Color(0xFF9CA3AF)),
            onPressed: onReply,
            constraints: const BoxConstraints(),
            padding: EdgeInsets.zero,
          ),
        ],
      ),
    );
  }
}
