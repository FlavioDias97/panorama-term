import React from 'react';

import type { BackgroundAction } from '~/domain/interfaces/plugin.interface';

import { getMinimapCorner, MINIMAP_SETTINGS_EVENT } from '~/usecase/util/minimap';

import styles from './styles.module.scss';

/** Mensagem que a página do fundo recebe quando uma ação é escolhida. */
export const BACKGROUND_ACTION = 'panorama:background-action';

interface Props {
  frame: React.RefObject<HTMLIFrameElement | null>;
  actions: BackgroundAction[];
  icon: string;
}

/** Botão no canto do canvas com as ações que o fundo do tema oferece. */
const BackdropActions = ({ frame, actions, icon }: Props): React.ReactElement => {
  const [open, setOpen] = React.useState(false);
  const [corner, setCorner] = React.useState(getMinimapCorner);
  const root = React.useRef<HTMLDivElement>(null);

  // o minimapa pode estar no mesmo canto e por cima; aí o botão fica ao lado dele
  React.useEffect(() => {
    const onChange = () => setCorner(getMinimapCorner());
    window.addEventListener(MINIMAP_SETTINGS_EVENT, onChange);
    return () => window.removeEventListener(MINIMAP_SETTINGS_EVENT, onChange);
  }, []);

  React.useEffect(() => {
    if (!open) return;
    const outside = (e: PointerEvent) => {
      if (!root.current?.contains(e.target as Node)) setOpen(false);
    };
    const esc = (e: KeyboardEvent) => e.key === 'Escape' && setOpen(false);
    window.addEventListener('pointerdown', outside, true);
    window.addEventListener('keydown', esc);
    return () => {
      window.removeEventListener('pointerdown', outside, true);
      window.removeEventListener('keydown', esc);
    };
  }, [open]);

  const send = (id: string) => {
    // a página roda com origem opaca (sandbox sem allow-same-origin), por isso o '*'
    frame.current?.contentWindow?.postMessage({ type: BACKGROUND_ACTION, id }, '*');
    setOpen(false);
  };

  return (
    <div
      ref={root}
      className={styles.root}
      data-beside-minimap={corner === 'bottom-right'}
      onPointerDown={(e) => e.stopPropagation()}
    >
      {open && (
        <div className={styles.menu} role="menu">
          {actions.map((a) => (
            <button key={a.id} type="button" role="menuitem" className={styles.item} onClick={() => send(a.id)}>
              {a.label}
            </button>
          ))}
        </div>
      )}
      <button
        type="button"
        className={styles.toggle}
        data-open={open}
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label="Ações do fundo"
        title="Ações do fundo"
        onClick={() => setOpen((v) => !v)}
      >
        {icon}
      </button>
    </div>
  );
};

export default BackdropActions;
