// Maps: hash tables that keep insertion order, laid out like Python's dict.
//
// `entries` holds the entries in the order their keys were first added;
// `index` is an open-addressing hash table of positions in `entries`. A
// removed entry stays in place, marked dead, until the entries are rebuilt
// (when they fill up), so removing keys while iterating is safe and order is
// kept. Keys and values are 64-bit payloads; when they are references (see
// the header's flags) the map holds one reference to each.
#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "qiq_runtime.h"

typedef struct {
    uint64_t key;
    uint64_t value;
    uint64_t hash;
    bool live;
} QiqEntry;

typedef struct {
    uint32_t key_type;     // 0 int, 1 double, 2 bool, 3 string, 4 char
    size_t length;         // live entries
    size_t used;           // entries in `entries`, live or dead
    size_t capacity;       // room in `entries`
    QiqEntry *entries;
    uint32_t *index;       // positions in `entries`, QIQ_EMPTY_SLOT for an empty slot
    size_t index_capacity; // a power of two, at least twice `capacity`
    uint64_t additions;    // counts keys added, to detect them during foreach
} QiqMap;

// The index holds 32-bit positions, half the size of 64-bit ones, so that
// more of a large map's index fits in the processor's cache: with a million
// keys it's 8 MB instead of 16 MB, and inserts and in-order lookups take a
// fifth less time. It limits a map to QIQ_MAX_ENTRIES entries, live or dead.
#define QIQ_EMPTY_SLOT UINT32_MAX
#define QIQ_MAX_ENTRIES ((size_t)UINT32_MAX)

static uint64_t qiq_mix(uint64_t value) {
    value ^= value >> 30;
    value *= UINT64_C(0xbf58476d1ce4e5b9);
    value ^= value >> 27;
    value *= UINT64_C(0x94d049bb133111eb);
    value ^= value >> 31;
    return value;
}

static uint64_t qiq_hash_bytes(const unsigned char *text) {
    uint64_t hash = UINT64_C(14695981039346656037);
    if (text == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, "null string used as map key");
    while (*text != 0) {
        hash ^= *text++;
        hash *= UINT64_C(1099511628211);
    }
    return hash;
}

static uint64_t qiq_hash_key(const QiqMap *map, uint64_t key) {
    switch (map->key_type) {
        case 1: {
            union { uint64_t bits; double number; } value = { .bits = key };
            if (isnan(value.number)) qiq_throw(QIQ_INVALID_ARGUMENT_EXCEPTION, "NaN cannot be used as a map key");
            if (value.number == 0.0) key = 0;
            return qiq_mix(key);
        }
        case 3:
        case 4:
            return qiq_hash_bytes((const unsigned char *)(uintptr_t)key);
        default:
            return qiq_mix(key);
    }
}

static int qiq_keys_equal(const QiqMap *map, uint64_t left, uint64_t right) {
    switch (map->key_type) {
        case 1: {
            union { uint64_t bits; double number; } a = { .bits = left };
            union { uint64_t bits; double number; } b = { .bits = right };
            if (isnan(a.number) || isnan(b.number)) return 0;
            return a.number == b.number;
        }
        case 3:
        case 4:
            return strcmp((const char *)(uintptr_t)left,
                          (const char *)(uintptr_t)right) == 0;
        default:
            return left == right;
    }
}

static QiqMap *qiq_require_map(void *handle, const char *message) {
    if (handle == NULL) qiq_throw(QIQ_RUNTIME_EXCEPTION, message);
    return (QiqMap *)handle;
}

// The live entry for `key`, or NULL.
static QiqEntry *qiq_find(QiqMap *map, uint64_t key, uint64_t hash) {
    size_t mask = map->index_capacity - 1;
    for (size_t slot = (size_t)hash & mask;; slot = (slot + 1) & mask) {
        uint32_t position = map->index[slot];
        if (position == QIQ_EMPTY_SLOT) return NULL;
        QiqEntry *entry = &map->entries[position];
        if (entry->live && entry->hash == hash && qiq_keys_equal(map, entry->key, key)) return entry;
    }
}

// Rebuilds the entries without dead ones (or with more room) and the index.
static void qiq_rebuild(QiqMap *map, size_t capacity) {
    if (capacity > QIQ_MAX_ENTRIES) {
        qiq_throw(QIQ_RUNTIME_EXCEPTION, "a map can hold at most 4294967295 entries");
    }
    size_t live = 0;
    for (size_t position = 0; position < map->used; position++) {
        if (map->entries[position].live) map->entries[live++] = map->entries[position];
    }
    if (capacity != map->capacity) {
        QiqEntry *grown = realloc(map->entries, capacity * sizeof *grown);
        if (grown == NULL) qiq_fatal("out of memory growing map");
        map->entries = grown;
        map->capacity = capacity;
    }
    map->used = live;
    size_t index_capacity = 16;
    while (index_capacity < capacity * 2) index_capacity *= 2;
    if (index_capacity != map->index_capacity) {
        free(map->index);
        map->index = malloc(index_capacity * sizeof *map->index);
        if (map->index == NULL) qiq_fatal("out of memory growing map");
        map->index_capacity = index_capacity;
    }
    memset(map->index, 0xff, index_capacity * sizeof *map->index);  // QIQ_EMPTY_SLOT
    size_t mask = index_capacity - 1;
    for (size_t position = 0; position < live; position++) {
        size_t slot = (size_t)map->entries[position].hash & mask;
        while (map->index[slot] != QIQ_EMPTY_SLOT) slot = (slot + 1) & mask;
        map->index[slot] = (uint32_t)position;
    }
}

// `value_references` is 1 when values are references, plus QIQ_MAY_CYCLE
// when they can be part of a cycle. Key types 3 and 4 (string and char) are
// references.
void *qiq_map_new(uint32_t key_type, int32_t value_references) {
    uint32_t flags = (key_type == 3 || key_type == 4 ? QIQ_KEYS_ARE_REFERENCES : 0)
        | (value_references & 1 ? QIQ_VALUES_ARE_REFERENCES : 0)
        | ((uint32_t)value_references & QIQ_MAY_CYCLE);
    QiqMap *map = qiq_allocate(sizeof(QiqMap), QIQ_KIND_MAP, flags);
    map->key_type = key_type;
    map->capacity = 8;
    map->entries = malloc(map->capacity * sizeof *map->entries);
    if (map->entries == NULL) qiq_fatal("out of memory creating map");
    qiq_rebuild(map, map->capacity);
    return map;
}

// Takes over one reference to `key` and `value` when they are references.
QIQ_GUARDED_VOID(qiq_map_set, (void *handle, uint64_t key, uint64_t value), (handle, key, value)) {
    QiqMap *map = qiq_require_map(handle, "cannot insert into an uninitialized map");
    uint32_t flags = QIQ_HEADER(map)->flags;
    uint64_t hash = qiq_hash_key(map, key);
    QiqEntry *entry = qiq_find(map, key, hash);
    if (entry != NULL) {
        // An existing key: keep the stored key, replace the value.
        if (flags & QIQ_KEYS_ARE_REFERENCES) qiq_release((void *)(uintptr_t)key);
        uint64_t old = entry->value;
        entry->value = value;
        if (flags & QIQ_VALUES_ARE_REFERENCES) qiq_release((void *)(uintptr_t)old);
        return;
    }
    if (map->used == map->capacity) {
        // Mostly dead entries: compact in place; otherwise grow.
        qiq_rebuild(map, map->length * 2 < map->used ? map->capacity : map->capacity * 2);
    }
    size_t position = map->used++;
    map->entries[position] = (QiqEntry){key, value, hash, true};
    size_t mask = map->index_capacity - 1;
    size_t slot = (size_t)hash & mask;
    while (map->index[slot] != QIQ_EMPTY_SLOT) slot = (slot + 1) & mask;
    map->index[slot] = (uint32_t)position;
    map->length++;
    map->additions++;
}

QIQ_GUARDED(uint64_t, qiq_map_get, 0, (void *handle, uint64_t key), (handle, key)) {
    QiqMap *map = qiq_require_map(handle, "cannot read from an uninitialized map");
    QiqEntry *entry = qiq_find(map, key, qiq_hash_key(map, key));
    if (entry == NULL) qiq_throw(QIQ_KEY_NOT_FOUND_EXCEPTION, "map key not found");
    return entry->value;
}

// For `map[key] ?? default`: whether the key is present, storing its value.
QIQ_GUARDED(bool, qiq_map_find, false, (void *handle, uint64_t key, uint64_t *value), (handle, key, value)) {
    QiqMap *map = qiq_require_map(handle, "cannot read from an uninitialized map");
    QiqEntry *entry = qiq_find(map, key, qiq_hash_key(map, key));
    if (entry == NULL) return false;
    *value = entry->value;
    return true;
}

// map_key_exists(key, map)
QIQ_GUARDED(bool, qiq_map_has, false, (void *handle, uint64_t key), (handle, key)) {
    QiqMap *map = qiq_require_map(handle, "map_key_exists: the map is null");
    return qiq_find(map, key, qiq_hash_key(map, key)) != NULL;
}

// unset(map[key]): removes the entry if present, releasing its key and value.
QIQ_GUARDED_VOID(qiq_map_remove, (void *handle, uint64_t key), (handle, key)) {
    QiqMap *map = qiq_require_map(handle, "unset: the map is null");
    QiqEntry *entry = qiq_find(map, key, qiq_hash_key(map, key));
    if (entry == NULL) return;
    uint32_t flags = QIQ_HEADER(map)->flags;
    entry->live = false;
    map->length--;
    // The index keeps pointing at the dead entry until the next rebuild, so
    // probe sequences through it still work.
    if (flags & QIQ_KEYS_ARE_REFERENCES) qiq_release((void *)(uintptr_t)entry->key);
    if (flags & QIQ_VALUES_ARE_REFERENCES) qiq_release((void *)(uintptr_t)entry->value);
}

// len() of a map: the number of entries.
QIQ_GUARDED(int64_t, qiq_len_map, 0, (void *handle), (handle)) {
    return (int64_t)qiq_require_map(handle, "len() of a null map")->length;
}

// ---------------------------------------------------------------------------
// Iteration, for foreach: positions 0 to qiq_map_iteration_end, skipping
// dead entries. Keys added during the loop would move entries when the map
// is rebuilt, so they make the loop throw (as in Python); removing keys and
// changing values are fine.

QIQ_GUARDED(int64_t, qiq_map_iteration_end, 0, (void *handle), (handle)) {
    return (int64_t)qiq_require_map(handle, "cannot iterate an uninitialized map")->used;
}

uint64_t qiq_map_version(void *handle) {
    return handle == NULL ? 0 : ((QiqMap *)handle)->additions;
}

QIQ_GUARDED(bool, qiq_map_entry_live, false, (void *handle, int64_t position, uint64_t version),
            (handle, position, version)) {
    QiqMap *map = (QiqMap *)handle;
    if (map->additions != version) {
        qiq_throw(QIQ_RUNTIME_EXCEPTION, "a key was added to a map during foreach over it");
    }
    return map->entries[position].live;
}

uint64_t qiq_map_key_at(void *handle, int64_t position) {
    return ((QiqMap *)handle)->entries[position].key;
}

uint64_t qiq_map_value_at(void *handle, int64_t position) {
    return ((QiqMap *)handle)->entries[position].value;
}

// ---------------------------------------------------------------------------
// map_keys, map_values, in_map

static void *qiq_map_column(QiqMap *map, bool keys, int64_t element_size, int32_t flags) {
    void *array = qiq_array_create((int64_t)map->length, element_size, flags);
    bool references = flags & 1;
    int64_t index = 0;
    for (size_t position = 0; position < map->used; position++) {
        QiqEntry *entry = &map->entries[position];
        if (!entry->live) continue;
        uint64_t payload = keys ? entry->key : entry->value;
        if (references) qiq_retain((void *)(uintptr_t)payload);
        if (element_size == 1) {
            ((uint8_t *)QIQ_ARRAY_DATA(array))[index++] = (uint8_t)payload;
        } else {
            ((uint64_t *)QIQ_ARRAY_DATA(array))[index++] = payload;
        }
    }
    return array;
}

// The keys or values in insertion order, as a new array; `flags` are the
// array's (see qiq_array_create).
QIQ_GUARDED(void *, qiq_map_keys, NULL, (void *handle, int64_t element_size, int32_t flags),
            (handle, element_size, flags)) {
    return qiq_map_column(qiq_require_map(handle, "map_keys: the map is null"), true, element_size, flags);
}

QIQ_GUARDED(void *, qiq_map_values, NULL, (void *handle, int64_t element_size, int32_t flags),
            (handle, element_size, flags)) {
    return qiq_map_column(qiq_require_map(handle, "map_values: the map is null"), false, element_size, flags);
}

// in_map(value, map). `kind` says how values compare: 0 by their bits
// (ints, bools, bytes), 1 as doubles, 2 as strings (by content), 3 by
// identity (arrays, maps, objects).
QIQ_GUARDED(bool, qiq_map_contains_value, false, (void *handle, uint64_t value, int32_t kind), (handle, value, kind)) {
    QiqMap *map = qiq_require_map(handle, "in_map: the map is null");
    for (size_t position = 0; position < map->used; position++) {
        QiqEntry *entry = &map->entries[position];
        if (!entry->live) continue;
        uint64_t stored = entry->value;
        bool equal;
        switch (kind) {
        case 1: {
            double a, b;
            memcpy(&a, &stored, sizeof a);
            memcpy(&b, &value, sizeof b);
            equal = a == b;
            break;
        }
        case 2:
            equal = stored == value
                || (stored != 0 && value != 0 && strcmp((const char *)(uintptr_t)stored, (const char *)(uintptr_t)value) == 0);
            break;
        default:
            equal = stored == value;
            break;
        }
        if (equal) return true;
    }
    return false;
}

// ---------------------------------------------------------------------------
// Used by the rest of the runtime.

void qiq_map_drop(void *handle) {
    QiqMap *map = (QiqMap *)handle;
    uint32_t flags = QIQ_HEADER(map)->flags;
    if (flags & (QIQ_KEYS_ARE_REFERENCES | QIQ_VALUES_ARE_REFERENCES)) {
        for (size_t position = 0; position < map->used; position++) {
            if (!map->entries[position].live) continue;
            if (flags & QIQ_KEYS_ARE_REFERENCES) qiq_release((void *)(uintptr_t)map->entries[position].key);
            if (flags & QIQ_VALUES_ARE_REFERENCES) qiq_release((void *)(uintptr_t)map->entries[position].value);
        }
    }
    free(map->entries);
    free(map->index);
}

void qiq_map_visit_values(void *handle, void (*visit)(void *value)) {
    QiqMap *map = (QiqMap *)handle;
    if (!(QIQ_HEADER(map)->flags & QIQ_MAY_CYCLE)) return;
    for (size_t position = 0; position < map->used; position++) {
        if (map->entries[position].live && map->entries[position].value != 0) {
            visit((void *)(uintptr_t)map->entries[position].value);
        }
    }
}

void qiq_map_free_collected(void *handle) {
    QiqMap *map = (QiqMap *)handle;
    uint32_t flags = QIQ_HEADER(map)->flags;
    bool values = (flags & QIQ_VALUES_ARE_REFERENCES) && !(flags & QIQ_MAY_CYCLE);
    for (size_t position = 0; position < map->used; position++) {
        if (!map->entries[position].live) continue;
        if (flags & QIQ_KEYS_ARE_REFERENCES) qiq_release((void *)(uintptr_t)map->entries[position].key);
        if (values) qiq_release((void *)(uintptr_t)map->entries[position].value);
    }
    free(map->entries);
    free(map->index);
}

size_t qiq_map_size(void *handle) {
    return ((QiqMap *)handle)->length;
}

uint32_t qiq_map_key_type(void *handle) {
    return ((QiqMap *)handle)->key_type;
}

// The live keys and values, in insertion order.
void qiq_map_collect(void *handle, uint64_t *keys, uint64_t *values) {
    QiqMap *map = (QiqMap *)handle;
    size_t count = 0;
    for (size_t position = 0; position < map->used; position++) {
        if (!map->entries[position].live) continue;
        keys[count] = map->entries[position].key;
        values[count] = map->entries[position].value;
        count++;
    }
}
