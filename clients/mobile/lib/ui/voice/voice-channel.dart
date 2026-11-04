import 'package:flutter/material.dart';

class VoiceChannelView extends StatefulWidget {
  final String channelName;
  final int participantsCount;

  const VoiceChannelView({
    Key? key,
    this.channelName = 'Reunión de equipo',
    this.participantsCount = 4,
  }) : super(key: key);

  @override
  State<VoiceChannelView> createState() => _VoiceChannelViewState();
}

class _VoiceChannelViewState extends State<VoiceChannelView> {
  bool _isMuted = false;
  bool _isVoiceUnavailable =
      true; // Simulando el aviso por defecto del prototipo

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: const Color(
        0xFFF8F9FA,
      ), // Estilo de fondo claro similar al tema
      body: Column(
        children: [
          // ================= Avisos (Ocultos/Visibles según estado) =================
          if (_isVoiceUnavailable)
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 16),
              child: Container(
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  color: const Color(0xFFFDE8E8),
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(color: const Color(0xFFF8B4B4)),
                ),
                child: Row(
                  children: [
                    const ContainerIconBadge(text: '!', isError: true),
                    const SizedBox(width: 12),
                    const Expanded(
                      child: Text(
                        'La voz no está disponible por el momento',
                        style: TextStyle(
                          color: Color(0xFF9B1C1C),
                          fontWeight: FontWeight.w500,
                        ),
                      ),
                    ),
                    ElevatedButton(
                      onPressed: () {
                        setState(() {
                          _isVoiceUnavailable = false; // Reintentar acción
                        });
                      },
                      style: ElevatedButton.styleFrom(
                        backgroundColor: Colors.white,
                        foregroundColor: const Color(0xFF374151),
                        elevation: 0,
                        side: const BorderSide(color: Color(0xFFD1D5DB)),
                      ),
                      child: const Text('Reintentar'),
                    ),
                  ],
                ),
              ),
            ),

          // ================= Participantes =================
          Expanded(
            child: Padding(
              padding: const EdgeInsets.all(24.0),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'PARTICIPANTES (${widget.participantsCount})',
                    style: const TextStyle(
                      fontSize: 12,
                      fontWeight: FontWeight.bold,
                      color: Color(0xFF6B7280),
                      letterSpacing: 0.8,
                    ),
                  ),
                  const SizedBox(height: 14),
                  Expanded(
                    child: GridView.count(
                      crossAxisCount: 2,
                      crossAxisSpacing: 16,
                      mainAxisSpacing: 16,
                      childAspectRatio: 1.6,
                      children: const [
                        ParticipantTile(
                          name: 'Luis Vázquez (tú)',
                          isMuted: false,
                          isSpeaking: true,
                          isSelf: true,
                        ),
                        ParticipantTile(
                          name: 'Ana López',
                          isMuted: true,
                          isSpeaking: false,
                        ),
                        ParticipantTile(
                          name: 'Carlos Ruiz',
                          isMuted: false,
                          isSpeaking: false,
                        ),
                        ParticipantTile(
                          name: 'María Torres',
                          isMuted: true,
                          isSpeaking: false,
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ),
          ),

          // ================= Controles de la llamada =================
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 14),
            decoration: const BoxDecoration(
              color: Colors.white,
              border: Border(top: BorderSide(color: Color(0xFFE5E7EB))),
            ),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                // Toggle de Silenciar
                OutlinedButton.icon(
                  onPressed: () {
                    setState(() {
                      _isMuted = !_isMuted;
                    });
                  },
                  icon: Icon(_isMuted ? Icons.mic_off : Icons.mic, size: 18),
                  label: Text(_isMuted ? 'Reactivar' : 'Silenciar'),
                  style: OutlinedButton.styleFrom(
                    foregroundColor: const Color(0xFF374151),
                    side: const BorderSide(color: Color(0xFFD1D5DB)),
                    padding: const EdgeInsets.symmetric(
                      horizontal: 20,
                      vertical: 12,
                    ),
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(8),
                    ),
                  ),
                ),
                const SizedBox(width: 16),
                // Botón de Salir
                ElevatedButton.icon(
                  onPressed: () {
                    Navigator.of(context).pop();
                  },
                  icon: const Icon(Icons.logout, size: 18),
                  label: const Text('Salir'),
                  style: ElevatedButton.styleFrom(
                    backgroundColor: const Color(
                      0xFFC84B10,
                    ), // Color primario de llamadas
                    foregroundColor: Colors.white,
                    padding: const EdgeInsets.symmetric(
                      horizontal: 24,
                      vertical: 12,
                    ),
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(8),
                    ),
                    elevation: 0,
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

// Subcomponente para la tarjeta de cada participante
class ParticipantTile extends StatelessWidget {
  final String name;
  final bool isMuted;
  final bool isSpeaking;
  final bool isSelf;

  const ParticipantTile({
    Key? key,
    required this.name,
    required this.isMuted,
    required this.isSpeaking,
    this.isSelf = false,
  }) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return Container(
      decoration: BoxDecoration(
        color: Colors.white,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: isSelf ? const Color(0xFF111827) : const Color(0xFFE5E7EB),
          width: isSelf ? 2 : 1,
        ),
      ),
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Stack(
            alignment: Alignment.center,
            children: [
              CircleAvatar(
                radius: 24,
                backgroundColor: const Color(0xFFF3F4F6),
                child: Text(
                  name.substring(0, 1),
                  style: const TextStyle(
                    fontWeight: FontWeight.bold,
                    color: Color(0xFF374151),
                  ),
                ),
              ),
              if (isSpeaking)
                Positioned.fill(
                  child: Container(
                    decoration: BoxDecoration(
                      shape: BoxShape.circle,
                      border: Border.all(color: Colors.green, width: 2),
                    ),
                  ),
                ),
            ],
          ),
          const SizedBox(height: 8),
          Text(
            name,
            style: const TextStyle(
              fontWeight: FontWeight.w600,
              fontSize: 13,
              color: Color(0xFF1F2937),
            ),
          ),
          const SizedBox(height: 4),
          Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(
                isMuted ? Icons.mic_off : Icons.mic,
                size: 14,
                color: isMuted ? Colors.red : Colors.grey,
              ),
              const SizedBox(width: 4),
              Text(
                isMuted ? 'Silenciado' : 'Hablando',
                style: TextStyle(
                  fontSize: 11,
                  color: isMuted ? Colors.red : Colors.grey[600],
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class ContainerIconBadge extends StatelessWidget {
  final String text;
  final bool isError;

  const ContainerIconBadge({
    Key? key,
    required this.text,
    required this.isError,
  }) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 24,
      height: 24,
      decoration: BoxDecoration(
        color: isError ? const Color(0xFFF8B4B4) : Colors.blue[100],
        shape: BoxShape.circle,
      ),
      child: Center(
        child: Text(
          text,
          style: TextStyle(
            color: isError ? const Color(0xFF9B1C1C) : Colors.blue[800],
            fontWeight: FontWeight.bold,
          ),
        ),
      ),
    );
  }
}
