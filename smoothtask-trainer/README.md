# SmoothTask Trainer

Python-инструменты для обучения CatBoostRanker и тюнинга политики SmoothTask.

## Установка

```bash
uv pip install -e .
```

## Использование

### Обучение ранкера

```bash
uv run smoothtask_trainer.train_ranker \
    --db /var/lib/smoothtask/snapshots.sqlite \
    --model-json model.json \
    --model-onnx model.onnx
```

