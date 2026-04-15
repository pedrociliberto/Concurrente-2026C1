## Clase 6 - Redes de Petri

### Red Ordinaria de Petri

Grafo bipartito que cumple con:

```
PN = (T,P,A)
```

- `T = t1, t2, ..., tn` es un conjunto de nodos llamados **transiciones**
- `P = p1, p2, ..., pn` es un conjunto de nodos llamados **lugares**
- `A c (T X P) U (P X T)` es un conjunto de arcos

Se establece que:

- `p_i` son los **estados del sistema**
- `t_i` son los **eventos que ocasionan cambios de estado**

#### Función de Marca

Se define la **función de marca** `M` que va de todos los Places, a los naturales (incluido el 0).

```
M: P -> N U 0
```

- En los Places se crean *tokens* (fichas), que pueden crearse/destruirse. 
- La marca inicial se corresponderá con el **estado inicial del sistema**.
- Cuando el token está en el lugar  `p1`, entonces `M(p1) = 1` y `M(p2) = 0`.
- Entonces `M_0 = (1,0)`
- El número que devuelve la función es la **cantidad de tokens que hay en ese Place**.

#### Funciones de Entrada y Salida

Sea `t ∈ PN = (T, P, A)` una transición, se definen las funciones:

- `I(t) = p / p ∈ P / (p,t) ∈ A` es la **entrada o input** de la transición `t`
- `I(t) = p / p ∈ P / (t,p) ∈ A` es la **salida o output** de la transición `t`

#### Grafo de alcance

Grafo dirigido formado por la **sucesión de funciones de marca** por las que va pasando el sistema.

#### Algunas interpretaciones

| Lugares de entrada  |       Transiciones       | Lugares de salida  |
| :-----------------: | :----------------------: | :----------------: |
|   Precondiciones    |         Eventos          |  Postcondiciones   |
|  Datos de entrada   |         Cómputos         |  Datos de salida   |
| Señales de entrada  | Procesamiento de señales | Señales de salida  |
| Bufferes de entrada |       Procesadores       | Bufferes de salida |
### Redes Generales de Petri

Grafo bipartito que cumple con:

```
PN = (T,P,A,W,M_0)
```

- `T = t1, t2, ..., tn` es un conjunto de nodos llamados **transiciones**
- `P = p1, p2, ..., pn` es un conjunto de nodos llamados **lugares**
- `A c (T X P) U (P X T)` es un conjunto de arcos
- `W : A → N` es la función de peso
- `M0 : P → N ∪ {0}` es la función de marca inicial

#### Reglas generales de disparo de transiciones

- La transición `t` está habilitada si y sólo si `M(p) ≥ W (p,t) : ∀p ∈ I(t)`
- Cuando t se dispara: 
	- `∀p ∈ I(t) : M(p) ← M(p) − W (p,t)`
	- `∀p' ∈ O(t) : M(p') ← M(p') + W (p',t)`
