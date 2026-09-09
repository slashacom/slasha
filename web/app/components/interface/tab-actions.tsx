import { createContext, useContext, useState } from 'react';
import { createPortal } from 'react-dom';

const TabActionsContext = createContext<HTMLElement | null>(null);

type TabActionsProviderProps = {
  children: (slot: React.ReactNode) => React.ReactNode;
};

export function TabActionsProvider(props: TabActionsProviderProps) {
  const { children } = props;
  const [element, setElement] = useState<HTMLElement | null>(null);

  return (
    <TabActionsContext.Provider value={element}>
      {children(<div ref={setElement} className="flex items-center gap-2" />)}
    </TabActionsContext.Provider>
  );
}

type TabActionsProps = {
  children: React.ReactNode;
};

export function TabActions(props: TabActionsProps) {
  const { children } = props;
  const element = useContext(TabActionsContext);

  if (!element) {
    return null;
  }

  return createPortal(children, element);
}
