import { useState, useEffect, useRef } from 'react';
import { selection_add, selection_remove, selection_clear } from '../commands';

export const useFileSelection = (fileList: string[]) => {
    const [selectedFiles, setSelectedFiles] = useState<string[]>([]);
    const prevSelectedFiles = useRef<string[]>([]);

    useEffect(() => {
        const prev = prevSelectedFiles.current;
        const next = selectedFiles;

        if (next.length === 0 && prev.length > 0) {
            selection_clear();
        } else {
            const addedFiles = next.filter((f) => !prev.includes(f));
            const removedFiles = prev.filter((f) => !next.includes(f));
            addedFiles.forEach((file) => selection_add(file));
            removedFiles.forEach((file) => selection_remove(file));
        }

        prevSelectedFiles.current = next;
    }, [selectedFiles]);

    const handleFileSelect = (file: string) => {
        setSelectedFiles((prev) =>
            prev.includes(file) ? prev.filter((f) => f !== file) : [...prev, file]
        );
    };

    const handleAddAll = () => {
        setSelectedFiles((prev) => [...new Set([...prev, ...fileList])]);
    };

    const handleSubtractAll = () => {
        setSelectedFiles((prev) => prev.filter((f) => !fileList.includes(f)));
    };

    const handleAsShown = () => {
        setSelectedFiles(fileList);
    };


    const handleClearSelection = () => {
        setSelectedFiles([]);
    };

    const setSelectionRange = (files: string[], mode: "add" | "remove") => {
        setSelectedFiles((prev) => {
            if (mode === "add") {
                return [...new Set([...prev, ...files])];
            } else {
                return prev.filter((f) => !files.includes(f));
            }
        });
    };

    return {
        selectedFiles,
        handleFileSelect,
        handleAddAll,
        handleSubtractAll,
        handleAsShown,
        handleClearSelection,
        setSelectionRange,
        setSelectedFiles,
    };
};