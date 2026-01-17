import { useEffect } from 'react';

export const useDocumentTitle = (title: string) => {
  useEffect(() => {
    const prevTitle = document.title;
    document.title = title;

    // Cleanup: restore previous title when component unmounts
    return () => {
      document.title = prevTitle;
    };
  }, [title]);
};
