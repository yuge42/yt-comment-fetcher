# yt-api-mockサーバーのページネーションバグ調査報告

## 問題の概要

ページネーショントークンを使用してgRPCストリームに再接続した後、コントロールエンドポイント経由で動的に追加されたメッセージがストリームに表示されません。

## 再現手順

1. `StreamList`でストリーミング開始（`page_token=None`）
2. 初期メッセージ5件を受信（msg-id-0 ～ msg-id-4）
3. 最後の`next_page_token`は"NQ=="（base64で"5"）
4. ストリームタイムアウト（5秒）後、`page_token="NQ=="`で再接続
5. `POST /control/chat_messages`で新規メッセージ3件を追加
6. ストリーム継続

**期待される動作**: 新規メッセージがストリームに表示される
**実際の動作**: 空レスポンス（`items: []`）が返される

## 技術的分析

### ページネーショントークンの解析
- トークンはbase64エンコードされた数値
- "NQ==" → "5"（メッセージインデックス）

### 根本原因

1. **メッセージIDの不一致**
   - 事前ロードメッセージ: `msg-id-0`, `msg-id-1`, ..., `msg-id-4`
   - 動的メッセージ: `test-message-{timestamp}-{index}`
   
2. **別々のメッセージストア**
   - モックサーバーは事前ロードメッセージと動的メッセージを別々に管理している可能性
   - ページネーション処理は事前ロードメッセージのみを対象としている

3. **ページネーションロジック**
   ```
   if page_token == "NQ==" (5):
       return messages where index > 5  // 事前ロードメッセージにはインデックス5以降が存在しない
   ```

### 証拠

テストログより:
```
Fetcher stderr: Reconnected successfully
Fetcher stderr: Received empty response (no items)

Creating message 1: POST https://yt-api-mock:8080/control/chat_messages
Message 1 created successfully
Creating message 2: POST https://yt-api-mock:8080/control/chat_messages
Message 2 created successfully
Creating message 3: POST https://yt-api-mock:8080/control/chat_messages
Message 3 created successfully

[再接続]
Fetcher stderr: Received empty response (no items)  // まだ空
```

## 影響範囲

### テスト不可能なシナリオ
- ダウンタイム中に追加されたメッセージが再接続後に表示される
- 動的メッセージでのページネーション継続
- ページネーショントークンによるメッセージ重複防止

### 現在の対処方法
テストを修正して:
- 再接続が成功することを検証
- ページネーショントークンが保持されることを検証  
- 空レスポンスがstderrにログされることを検証
- 動的メッセージの表示は期待しない

## 推奨される修正方法

### モックサーバー側（yt-api-mock）

1. **統一メッセージキューの実装**
   - 事前ロードと動的メッセージを単一のシーケンスで管理
   - 動的メッセージに連続したシーケンス番号を割り当て

2. **コントロールエンドポイントの更新**
   - 新規メッセージに適切なシーケンス番号を付与
   - アクティブなストリームに新規メッセージを通知

3. **ページネーションロジックの修正**
   ```
   if page_token:
       sequence = decode_token(page_token)
       return all_messages.filter(msg => msg.sequence > sequence)
   ```

## yt-api-mockリポジトリ用のイシューテンプレート

**タイトル**: Dynamic messages added via control endpoint don't appear in paginated streams

**説明**:
ページネーショントークンを使用したStreamListエンドポイントで、`/control/chat_messages`経由で動的に追加されたメッセージが後続のストリームレスポンスに表示されません。

**再現手順**:
1. `StreamList(page_token=None)`でストリーミング開始
2. 初期メッセージを受信（例: msg-id-0 ～ msg-id-4）
3. 最後の`next_page_token`を記録（例: "NQ=="はbase64で"5"）
4. `StreamList(page_token="NQ==")`で再接続
5. `POST /control/chat_messages`で新規メッセージを追加
6. 同じページネーショントークンでストリーミング継続

**期待される動作**:
新規メッセージがストリームレスポンスに表示される

**実際の動作**:
ストリームは空の`items`配列を返す

**環境**:
- モックサーバーバージョン: dev-d18c431
- 設定: CHAT_STREAM_TIMEOUT=5

**影響**:
ライブチャットの一般的なシナリオである、動的に到着するメッセージとのページネーション動作をテストできません。

**提案する解決策**:
事前ロードメッセージと動的メッセージの両方で機能する統一メッセージシーケンスを維持し、ページネーショントークンが両ソースで正しく動作するようにする。

## 参考資料

詳細な技術分析: `MOCK_SERVER_PAGINATION_INVESTIGATION.md`
