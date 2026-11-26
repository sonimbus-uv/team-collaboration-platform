import 'package:flutter/material.dart';

import '../theme/app-theme-extension.dart';

class VoiceChannelView extends StatefulWidget {
  final String channelName;
  final int participantsCount;

  const VoiceChannelView({
    super.key,
    this.channelName = 'Reunión de equipo',
    this.participantsCount = 4,
  });

  @override
  State<VoiceChannelView> createState() => _VoiceChannelViewState();
}

class _VoiceChannelViewState extends State<VoiceChannelView> {
  bool _isMuted = false;
  bool _isVoiceUnavailable =
      true; // Simulando el aviso por defecto del prototipo

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final themeColors = theme.extension<AppThemeColors>()!;

    return Scaffold(
      backgroundColor: colorScheme.surface,
      body: Column(
        children: [
          // ================= Avisos (Ocultos/Visibles según estado) =================
          if (_isVoiceUnavailable)
            Padding(
              padding: const EdgeInsets.fromLTRB(24, 16, 24, 0),
              child: Container(
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  color: themeColors.errorBackground,
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(
                    color: colorScheme.error.withValues(alpha: 0.45),
                  ),
                ),
                child: Row(
                  children: [
                    const ContainerIconBadge(text: '!', isError: true),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Text(
                        'La voz no está disponible por el momento',
                        style: TextStyle(
                          color: colorScheme.error,
                          fontWeight: FontWeight.w500,
                        ),
                      ),
                    ),
                    const SizedBox(width: 8),
                    OutlinedButton(
                      onPressed: () {
                        setState(() {
                          _isVoiceUnavailable = false;
                        });
                      },
                      style: OutlinedButton.styleFrom(
                        backgroundColor: colorScheme.surface,
                        foregroundColor: colorScheme.onSurface,
                        side: BorderSide(color: colorScheme.outlineVariant),
                        shape: RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(8),
                        ),
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
                    style: TextStyle(
                      fontSize: 12,
                      fontWeight: FontWeight.bold,
                      color: colorScheme.onSurfaceVariant,
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
                      children: [
                        ParticipantTile(
                          name: 'Luis Vázquez (tú)',
                          isMuted: _isMuted,
                          isSpeaking: !_isMuted,
                          isSelf: true,
                        ),
                        const ParticipantTile(
                          name: 'Ana López',
                          isMuted: true,
                          isSpeaking: false,
                        ),
                        const ParticipantTile(
                          name: 'Carlos Ruiz',
                          isMuted: false,
                          isSpeaking: false,
                        ),
                        const ParticipantTile(
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
            decoration: BoxDecoration(
              color: colorScheme.surface,
              border: Border(top: BorderSide(color: themeColors.divider)),
            ),
            child: SafeArea(
              top: false,
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
                    icon: Icon(
                      _isMuted ? Icons.mic_off : Icons.mic,
                      size: 18,
                      color: _isMuted
                          ? colorScheme.error
                          : colorScheme.onSurface,
                    ),
                    label: Text(_isMuted ? 'Reactivar' : 'Silenciar'),
                    style: OutlinedButton.styleFrom(
                      foregroundColor: _isMuted
                          ? colorScheme.error
                          : colorScheme.onSurface,
                      backgroundColor: _isMuted
                          ? themeColors.errorBackground
                          : Colors.transparent,
                      side: BorderSide(
                        color: _isMuted
                            ? colorScheme.error.withValues(alpha: 0.45)
                            : colorScheme.outlineVariant,
                      ),
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
                      backgroundColor: colorScheme.primary,
                      foregroundColor: colorScheme.onPrimary,
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
    super.key,
    required this.name,
    required this.isMuted,
    required this.isSpeaking,
    this.isSelf = false,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final themeColors = theme.extension<AppThemeColors>()!;

    return Container(
      decoration: BoxDecoration(
        color: themeColors.sidePanel,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: isSelf ? colorScheme.primary : themeColors.divider,
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
                backgroundColor: isSelf
                    ? colorScheme.primaryContainer
                    : themeColors.selectedItem,
                child: Text(
                  name.isNotEmpty ? name.substring(0, 1) : '?',
                  style: TextStyle(
                    fontWeight: FontWeight.bold,
                    color: isSelf
                        ? colorScheme.onPrimaryContainer
                        : colorScheme.onSurface,
                  ),
                ),
              ),
              if (isSpeaking)
                Positioned.fill(
                  child: Container(
                    decoration: BoxDecoration(
                      shape: BoxShape.circle,
                      border: Border.all(
                        color: themeColors.mintForeground,
                        width: 2,
                      ),
                    ),
                  ),
                ),
            ],
          ),
          const SizedBox(height: 8),
          Text(
            name,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: TextStyle(
              fontWeight: FontWeight.w600,
              fontSize: 13,
              color: colorScheme.onSurface,
            ),
          ),
          const SizedBox(height: 4),
          Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(
                isMuted ? Icons.mic_off : Icons.mic,
                size: 14,
                color: isMuted
                    ? colorScheme.error
                    : (isSpeaking
                          ? themeColors.mintForeground
                          : colorScheme.onSurfaceVariant),
              ),
              const SizedBox(width: 4),
              Text(
                isMuted ? 'Silenciado' : (isSpeaking ? 'Hablando' : 'En línea'),
                style: TextStyle(
                  fontSize: 11,
                  fontWeight: isSpeaking ? FontWeight.w600 : FontWeight.normal,
                  color: isMuted
                      ? colorScheme.error
                      : (isSpeaking
                            ? themeColors.mintForeground
                            : colorScheme.onSurfaceVariant),
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
    super.key,
    required this.text,
    required this.isError,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final themeColors = theme.extension<AppThemeColors>()!;

    return Container(
      width: 24,
      height: 24,
      decoration: BoxDecoration(
        color: isError
            ? colorScheme.errorContainer
            : themeColors.mintBackground,
        shape: BoxShape.circle,
      ),
      child: Center(
        child: Text(
          text,
          style: TextStyle(
            color: isError
                ? colorScheme.onErrorContainer
                : themeColors.mintForeground,
            fontWeight: FontWeight.bold,
          ),
        ),
      ),
    );
  }
}
