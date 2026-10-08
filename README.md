# Rust RAG with FastEmbed

ตัวอย่าง RAG แบบ CLI: Rust ใช้ FastEmbed รุ่น `multilingual-e5-small` สร้าง embedding, Qdrant เก็บและค้นเวกเตอร์, Ollama สร้างคำตอบจากข้อความที่ค้นได้ รองรับไฟล์ UTF-8 `.txt` และ `.md` ใน `docs/` (ค้นไฟล์ในโฟลเดอร์ย่อยด้วย)

ดู [RAG flow](docs/rag-flow.md) สำหรับเส้นทาง ingest และถามตอบ ไฟล์นี้ถูกข้ามเมื่อสั่ง ingest เอกสาร

## เริ่มใช้งาน

ต้องมี Docker และ Docker Compose; ครั้งแรกจะดาวน์โหลด Docker images, embedding model และ chat model

```sh
make setup
make chat q="ร้านเปิดกี่โมง และลาเต้เย็นราคาเท่าไร?"
```

เพิ่มไฟล์ใน `docs/` แล้วรัน `make ingest` อีกครั้งได้ หรือระบุไฟล์เดียวด้วย `make ingest DOCS=/docs/example.md` การ ingest ไฟล์เดิมจะลบ chunks รุ่นก่อนแล้วแทนที่ด้วยรุ่นใหม่ คำตอบจะแสดงหมายเลขอ้างอิงและชื่อไฟล์ต้นทาง หากใช้ `make chat q=""` จะมีช่องให้พิมพ์คำถาม

ปรับรุ่น Ollama ได้ผ่าน `OLLAMA_MODEL` เช่น `OLLAMA_MODEL=qwen2.5:3b make pull-model` แล้วใช้ `OLLAMA_MODEL=qwen2.5:3b make chat q="..."`

```sh
make down
```

ข้อมูล Qdrant, โมเดล Ollama และ FastEmbed เก็บใน Docker volumes; `docker compose down -v` จะลบข้อมูลเหล่านี้

## พัฒนาบนเครื่อง

```sh
cargo test
QDRANT_URL=http://localhost:6333 OLLAMA_URL=http://localhost:11435 cargo run -- ingest docs
```

การแบ่งข้อความในตัวอย่างนี้ใช้จำนวนอักขระ 500 ตัวและซ้อนทับ 80 ตัว เหมาะสำหรับสาธิตกับไฟล์ข้อความสั้น ๆ; ยังไม่มีตัวอ่าน PDF, การควบคุมสิทธิ์ หรือการจัดการไฟล์ที่ถูกลบจาก `docs/`
