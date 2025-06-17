import { Worker, isMainThread, parentPort, workerData } from 'worker_threads';
import ExcelJS from 'exceljs';

if (!isMainThread) {
    // Worker thread code
    async function processXlsxBuffer(buffer) {
        const workbook = new ExcelJS.Workbook();
        await workbook.xlsx.load(buffer);
        
        let fullText = '';
        
        workbook.eachSheet((worksheet, sheetId) => {
            const sheetName = worksheet.name;
            fullText += `Sheet: ${sheetName}\n\n`;
            
            const csvRows = [];
            worksheet.eachRow({ includeEmpty: true }, (row, rowNumber) => {
                const csvCells = [];
                row.eachCell({ includeEmpty: true }, (cell, colNumber) => {
                    const value = cell.value;
                    let cellText = '';
                    
                    if (value === null || value === undefined) {
                        cellText = '';
                    } else if (typeof value === 'object') {
                        if (value.formula) {
                            cellText = value.result?.toString() || '';
                        } else if (value.hyperlink) {
                            cellText = value.text || value.hyperlink;
                        } else if (value.richText && Array.isArray(value.richText)) {
                            cellText = value.richText.map(rt => rt.text || '').join('');
                        } else if (value.error) {
                            cellText = `#${value.error}`;
                        } else if (value instanceof Date) {
                            cellText = value.toISOString().split('T')[0];
                        } else if (value.text !== undefined) {
                            cellText = value.text.toString();
                        } else if (value.result !== undefined) {
                            cellText = value.result.toString();
                        } else {
                            cellText = JSON.stringify(value);
                        }
                    } else {
                        cellText = value.toString();
                    }
                    
                    if (cellText.includes(',') || cellText.includes('"') || cellText.includes('\n')) {
                        cellText = `"${cellText.replace(/"/g, '""')}"`;
                    }
                    csvCells.push(cellText);
                });
                csvRows.push(csvCells.join(','));
            });
            
            fullText += csvRows.join('\n') + '\n\n';
        });
        
        return fullText;
    }
    
    // Listen for messages from main thread
    parentPort.on('message', async (data) => {
        try {
            const { buffer } = data;
            const result = await processXlsxBuffer(buffer);
            parentPort.postMessage({ success: true, result });
        } catch (error) {
            parentPort.postMessage({ success: false, error: error.message });
        }
    });
}

// Helper function to run XLSX processing in worker thread
export async function processXlsxInWorker(buffer) {
    return new Promise((resolve, reject) => {
        const worker = new Worker(new URL(import.meta.url), {
            type: 'module'
        });
        
        worker.postMessage({ buffer });
        
        worker.on('message', (data) => {
            worker.terminate();
            if (data.success) {
                resolve(data.result);
            } else {
                reject(new Error(data.error));
            }
        });
        
        worker.on('error', (error) => {
            worker.terminate();
            reject(error);
        });
        
        worker.on('exit', (code) => {
            if (code !== 0) {
                reject(new Error(`Worker stopped with exit code ${code}`));
            }
        });
    });
}