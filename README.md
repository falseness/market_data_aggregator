# market_data_aggregator

(как запускать ниже)

## Идея алгоритма

Храним агрегированный l2 как Vec<Price, Amount>, а исходный - BTreeMap<Price, Amount>.

Предполагаем, что в векторе будет всегда мало элементов (<= 15).

Если add_quote изменение в исходном l2, находим соответствующий индекс в Vec и обновляем amount за const. 
Если теперь quotes больше необходимого (то есть без последней quote всё равно набирается достаточно amount), переходим к следующему индексу и "переливаем" quotes на следующий уровень.
Если после всего этого избыток quotes в следущем уровне, проводим ту же операцию для него.

Итого в худшем случае пройдемся по 15 уровням + запросы в BTreeMap. Ожидаю, что мало элементов перельется из одного уровня на другой. т.е. несколько элементов * 15 * log(размера исходного l2) = O(15 * logn) = O(logn) 

где n - количество элементов в BTreeMap.

remove_quote аналогично: ищем соответствующий индекс, обновляем price amount.
Если amount меньше необходимого, переходим к следующему индексу и переливаем quotes с предыдущего уровня.
Если недостаток amount на следующем уровне, проводим ту же операцию для него.
Аналогично получаем O(15 * logn) = O(logn)

Ещё нужно удовлетворить требование на depth. Во-первых, храним max_depth_price - цену на глубине depth.

Используя max_depth_price, прекращаем переливания в add_quote и remove_quote.

Ещё нужно max_depth_price обновлять. Это делается запросом в BTreeMap + обновлением последнего элемента в векторе.

Основная логика находится в `src/solutions/fast.rs`

## Как убедиться, что код работает

Помимо описанного решения написал ещё медленное, чтобы сравнить результаты.

В tests есть стресс тесты

Запуск:
```bash
cargo test
```

Результат исполнения тестов:

```bash
     Running tests/fast_solution_test.rs (target/debug/deps/fast_solution_test-a48a9bef4a479007)

running 4 tests
test tests::test_from_problem_statement ... ok
test tests::test_simple_with_removes ... ok
test tests::test_stress_bid ... ok
test tests::test_stress_ask ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.11s

   Doc-tests market_data_aggregator

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

## Проверить скорость:

Скорость проверяется по фикстуре. Пробегаемся по всей json много раз, чтобы иметь стабильный результат.

Чтобы не работать с вещественными числами, умножаю их на 1e8

Билд `nightly`, потому что использую Cursor (aka итератор для мапы).

Можно ещё ускорить в константу раз, т.к. я не экономил запросы в map'у. Работая с Cursor, можно их значительно уменьшить, но код станет сильно сложнее.

```bash
cargo +nightly build --release
./target/release/market_data_aggregator
```

Результат исполнения программы:
```
Fast solution: 
Time taken: 4.42s

Slow obvious solution:
Time taken: 15.58s
```
