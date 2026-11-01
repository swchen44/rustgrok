/*
 * Licensed to the Apache Software Foundation (ASF) under one or more
 * contributor license agreements.  See the NOTICE file distributed with
 * this work for additional information regarding copyright ownership.
 * The ASF licenses this file to You under the Apache License, Version 2.0
 * (the "License"); you may not use this file except in compliance with
 * the License.  You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
// SPDX-License-Identifier: Apache-2.0
// Restricted single-term G1 adaptation of Lucene 9.12.3 BM25Similarity.java,
// commit f965e930673c1f5cb478dc6a8907f5fc4ef7b539, lines 93-99, 161-172,
// 194-217. See ../LICENSE-APACHE-2.0.txt and ../LUCENE-NOTICE.txt.
// This is not a general Lucene query, norm, collector or index implementation.

pub fn score(doc_count: u64, doc_freq: u64, total_tokens: u64, length: u32, freq: u32) -> f32 {
    let idf =
        (1.0_f64 + ((doc_count - doc_freq) as f64 + 0.5) / (doc_freq as f64 + 0.5)).ln() as f32;
    let average = (total_tokens as f64 / doc_count as f64) as f32;
    let k1 = 1.2_f32;
    let b = 0.75_f32;
    let inverse = 1.0_f32 / (k1 * ((1.0_f32 - b) + b * length as f32 / average));
    idf - idf / (1.0_f32 + freq as f32 * inverse)
}
