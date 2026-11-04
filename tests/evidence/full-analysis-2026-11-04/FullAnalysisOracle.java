/*
 * CDDL HEADER START
 * The contents of this file are subject to the terms of the Common
 * Development and Distribution License (the "License"). You may not use
 * this file except in compliance with the License. See LICENSE.txt.
 * When distributing Covered Code, include this CDDL HEADER and LICENSE.txt.
 * CDDL HEADER END
 */
/*
 * Copyright (c) 2005, 2025, Oracle and/or its affiliates. All rights reserved.
 * Portions Copyright (c) 2017, 2020, Chris Fraire <cfraire@me.com>.
 * Portions Copyright (c) 2026, swchen44.
 * Adapted as a fixed-version diagnostic from the original analyzer API and
 * earlier RustgrokTokenProbe; upstream 1.14.20 commit
 * 3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3.
 */
// SPDX-License-Identifier: CDDL-1.0
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import org.apache.lucene.analysis.Analyzer;
import org.apache.lucene.analysis.TokenStream;
import org.apache.lucene.analysis.tokenattributes.CharTermAttribute;
import org.apache.lucene.analysis.tokenattributes.OffsetAttribute;
import org.apache.lucene.analysis.tokenattributes.PositionIncrementAttribute;
import org.opengrok.indexer.analysis.CompatibleAnalyser;
import org.opengrok.indexer.analysis.c.CAnalyzerFactory;
import org.opengrok.indexer.analysis.python.PythonAnalyzerFactory;
import org.opengrok.indexer.analysis.javascript.JavaScriptAnalyzerFactory;

public class FullAnalysisOracle {
    static String hexUtf16(String text) {
        StringBuilder out = new StringBuilder();
        for (int i = 0; i < text.length(); ++i) out.append(String.format("%04x", (int) text.charAt(i)));
        return out.toString();
    }
    static Analyzer analyzer(String name) {
        switch (name) {
            case "c": return new CAnalyzerFactory().getAnalyzer();
            case "python": return new PythonAnalyzerFactory().getAnalyzer();
            case "javascript": return new JavaScriptAnalyzerFactory().getAnalyzer();
            case "query": return new CompatibleAnalyser();
            default: throw new IllegalArgumentException("unknown analyzer");
        }
    }
    public static void main(String[] args) throws Exception {
        if (args.length != 3) throw new IllegalArgumentException("mode analyzer path");
        String text = Files.readString(Path.of(args[2]), StandardCharsets.UTF_8);
        try (Analyzer a = analyzer(args[1])) {
            if (args[0].equals("normalize")) {
                System.out.println("N\t" + hexUtf16(a.normalize("full", text).utf8ToString()));
            } else if (args[0].equals("stream")) {
                try (TokenStream stream = a.tokenStream("full", text)) {
                    CharTermAttribute term = stream.addAttribute(CharTermAttribute.class);
                    OffsetAttribute offset = stream.addAttribute(OffsetAttribute.class);
                    PositionIncrementAttribute inc = stream.addAttribute(PositionIncrementAttribute.class);
                    stream.reset(); int position = -1;
                    while (stream.incrementToken()) {
                        position += inc.getPositionIncrement();
                        System.out.println("T\t" + position + "\t" + offset.startOffset() + "\t" + offset.endOffset() + "\t" + hexUtf16(term.toString()));
                    }
                    stream.end(); System.out.println("END\t" + offset.endOffset());
                }
            } else throw new IllegalArgumentException("unknown mode");
        }
    }
}
