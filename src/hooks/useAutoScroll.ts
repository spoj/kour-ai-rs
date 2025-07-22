import { useEffect, useRef } from 'react';

export const useAutoScroll = (dependencies: any[]) => {
    const scrollRef = useRef<HTMLDivElement>(null);
    const prevDepsLength = useRef(dependencies.length);

    useEffect(() => {
        if (dependencies.length > prevDepsLength.current) {
            setTimeout(() => {
                if (scrollRef.current) {
                    scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
                }
            }, 100);
        }
        prevDepsLength.current = dependencies.length;
    }, [dependencies]);

    return scrollRef;
};