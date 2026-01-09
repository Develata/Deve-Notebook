/**
 * Parses the document to find Math ranges ($...$ and $$...$$).
 * Ignores escaped dollars (\$).
 * Returns a list of token objects or ranges.
 */
export function findMathRanges(docString) {
  const mathRanges = [];
  const regexAnyDollar = /\$+/g;
  
  const isEscaped = (index) => {
    let backslashes = 0;
    let i = index - 1;
    while (i >= 0 && docString[i] === "\\") {
      backslashes++;
      i--;
    }
    return backslashes % 2 === 1;
  };

  let match;
  let tokens = [];

  while ((match = regexAnyDollar.exec(docString)) !== null) {
    const val = match[0];
    const index = match.index;

    if (isEscaped(index)) continue;

    if (val === "$$" || val === "$") {
      tokens.push({ type: val, index });
    }
  }

  let mode = "NONE";
  let startToken = null;

  for (let i = 0; i < tokens.length; i++) {
    let t = tokens[i];

    if (mode === "NONE") {
      if (t.type === "$$") {
        mode = "BLOCK";
        startToken = t;
      } else if (t.type === "$") {
        mode = "INLINE";
        startToken = t;
      }
    } else if (mode === "BLOCK") {
      if (t.type === "$$") {
        // Block math found
        mathRanges.push({ 
            type: "BLOCK",
            from: startToken.index, 
            to: t.index + 2,
            contentFrom: startToken.index + 2,
            contentTo: t.index
        });
        mode = "NONE";
        startToken = null;
      }
    } else if (mode === "INLINE") {
       // Check for double newline break
       let contentSoFar = docString.slice(startToken.index + 1, t.index);
       if (contentSoFar.includes("\n\n")) {
         mode = "NONE";
         startToken = null;
         i--; // Re-evaluate current token in NONE mode
         continue;
       }

       if (t.type === "$") {
         // Inline match found
         mathRanges.push({
             type: "INLINE",
             from: startToken.index,
             to: t.index + 1,
             contentFrom: startToken.index + 1,
             contentTo: t.index
         });
         mode = "NONE";
         startToken = null;
       } else if (t.type === "$$") {
         // Encountered $$ while in inline mode, reset
         mode = "NONE";
         startToken = null;
         i--;
         continue;
       }
    }
  }

  return mathRanges;
}
