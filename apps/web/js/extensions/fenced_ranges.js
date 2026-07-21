/**
 * Scan GFM-style fenced code blocks without creating renderer state.
 * Returned ranges use half-open UTF-16 offsets.
 */
export function scanFencedRanges(doc) {
    const lines = documentLines(doc);
    const ranges = [];

    for (let i = 0; i < lines.length; i++) {
        const opener = parseFenceOpener(lines[i].text);
        if (!opener) continue;

        let closingLine = null;
        let closingIndex = i + 1;
        for (; closingIndex < lines.length; closingIndex++) {
            if (isFenceCloser(lines[closingIndex].text, opener)) {
                closingLine = lines[closingIndex];
                break;
            }
        }

        const contentFrom = lines[i].breakTo;
        const contentTo = closingLine ? closingLine.from : doc.length;
        ranges.push({
            from: lines[i].from,
            to: closingLine ? closingLine.to : doc.length,
            contentFrom,
            contentTo,
            marker: opener.marker,
            markerLength: opener.length,
            infoString: opener.infoString,
            closed: closingLine !== null,
        });

        i = closingLine ? closingIndex : lines.length;
    }

    return ranges;
}
function parseFenceOpener(line) {
    const match = /^( {0,3})(`{3,}|~{3,})(.*)$/.exec(line);
    if (!match) return null;

    const run = match[2];
    const marker = run[0];
    const suffix = match[3];
    if (marker === "`" && suffix.includes("`")) return null;

    return {
        marker,
        length: run.length,
        infoString: suffix.trim(),
    };
}

function isFenceCloser(line, opener) {
    const match = /^( {0,3})(`+|~+)[\t ]*$/.exec(line);
    return !!match
        && match[2][0] === opener.marker
        && match[2].length >= opener.length;
}

function documentLines(doc) {
    const lines = [];
    let from = 0;

    while (from < doc.length) {
        let to = from;
        while (to < doc.length && doc[to] !== "\n" && doc[to] !== "\r") to++;

        let breakTo = to;
        if (breakTo < doc.length && doc[breakTo] === "\r") breakTo++;
        if (breakTo < doc.length && doc[breakTo] === "\n") breakTo++;

        lines.push({ from, to, breakTo, text: doc.slice(from, to) });
        from = breakTo;
    }

    if (doc.length === 0 || /[\r\n]$/.test(doc)) {
        lines.push({ from: doc.length, to: doc.length, breakTo: doc.length, text: "" });
    }
    return lines;
}
