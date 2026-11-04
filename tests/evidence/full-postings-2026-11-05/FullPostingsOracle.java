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
import java.io.*;
import java.lang.reflect.Method;
import java.nio.file.*;
import org.apache.lucene.analysis.*;
import org.apache.lucene.analysis.tokenattributes.*;
import org.apache.lucene.document.*;
import org.apache.lucene.index.*;
import org.apache.lucene.search.*;
import org.apache.lucene.store.*;
import org.opengrok.indexer.analysis.plain.PlainAnalyzer;
import org.opengrok.indexer.analysis.c.CAnalyzerFactory;
import org.opengrok.indexer.analysis.python.PythonAnalyzerFactory;
import org.opengrok.indexer.analysis.javascript.JavaScriptAnalyzerFactory;
import org.opengrok.indexer.search.CustomQueryParser;

public class FullPostingsOracle {
    static String hex(String s) {
        StringBuilder b = new StringBuilder();
        for (int i=0;i<s.length();i++) b.append(String.format("%04x", (int)s.charAt(i)));
        return b.toString();
    }
    static PlainAnalyzer analyzer(String name) {
        return (PlainAnalyzer) switch(name) {
            case "c" -> new CAnalyzerFactory().getAnalyzer();
            case "python" -> new PythonAnalyzerFactory().getAnalyzer();
            case "javascript" -> new JavaScriptAnalyzerFactory().getAnalyzer();
            default -> throw new IllegalArgumentException(name);
        };
    }
    public static void main(String[] args) throws Exception {
        try (PlainAnalyzer a=analyzer(args[0]); Directory dir=new ByteBuffersDirectory()) {
            Method method=PlainAnalyzer.class.getDeclaredMethod("getReader", InputStream.class);
            method.setAccessible(true);
            String text;
            try (InputStream in=Files.newInputStream(Path.of(args[1])); Reader r=(Reader)method.invoke(a,in)) {
                StringWriter out=new StringWriter(); r.transferTo(out); text=out.toString();
            }
            System.out.println("READER\t"+hex(text));
            try(TokenStream ts=a.tokenStream("full",text)) {
                var term=ts.addAttribute(CharTermAttribute.class);
                var offset=ts.addAttribute(OffsetAttribute.class);
                var inc=ts.addAttribute(PositionIncrementAttribute.class);
                int pos=-1; ts.reset();
                while(ts.incrementToken()) {
                    pos+=inc.getPositionIncrement();
                    System.out.println("T\t"+pos+"\t"+offset.startOffset()+"\t"+offset.endOffset()+"\t"+hex(term.toString()));
                }
                ts.end();
            }
            FieldType type=new FieldType(TextField.TYPE_NOT_STORED);
            type.setIndexOptions(IndexOptions.DOCS_AND_FREQS_AND_POSITIONS_AND_OFFSETS); type.freeze();
            try(IndexWriter writer=new IndexWriter(dir,new IndexWriterConfig(a))) {
                Document d=new Document(); d.add(new Field("full",text,type)); writer.addDocument(d); writer.commit();
            }
            try(DirectoryReader r=DirectoryReader.open(dir)) {
                IndexSearcher searcher=new IndexSearcher(r);
                for(int i=2;i<args.length;i++) {
                    Query q=new CustomQueryParser("full").parse(args[i]);
                    System.out.println("QUERY\t"+hex(args[i])+"\t"+q.getClass().getName()+"\t"+q+"\t"+searcher.count(q));
                    if(q instanceof TermQuery tq) {
                        PostingsEnum p=MultiTerms.getTermPostingsEnum(r,"full",tq.getTerm().bytes(),PostingsEnum.ALL);
                        if(p!=null) while(p.nextDoc()!=DocIdSetIterator.NO_MORE_DOCS) {
                            int n=p.freq();
                            for(int j=0;j<n;j++) {
                                int pos=p.nextPosition();
                                System.out.println("P\t"+hex(args[i])+"\t"+p.docID()+"\t"+pos+"\t"+p.startOffset()+"\t"+p.endOffset());
                            }
                        }
                    }
                }
            }
        }
    }
}
