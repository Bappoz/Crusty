/*
 * Programa-demo para a apresentação final (issue #163).
 *
 * Mostra, em poucas linhas, os recursos centrais já estáveis do
 * subconjunto de C suportado pelo compilador:
 *   - variaveis e expressoes aritmeticas
 *   - funcoes com chamada recursiva (factorial)
 *   - controle de fluxo: if/else e while
 *
 * Saida esperada: exit code 80
 *   factorial(4)          = 24
 *   sum_even_squares(6)   = 2*2 + 4*4 + 6*6 = 56
 *   24 + 56               = 80
 */

int factorial(int n) {
    if (n <= 1) {
        return 1;
    }
    return n * factorial(n - 1);
}

int sum_even_squares(int n) {
    int total = 0;
    int i = 1;
    while (i <= n) {
        if (i % 2 == 0) {
            total = total + i * i;
        }
        i = i + 1;
    }
    return total;
}

int main(void) {
    int fact4 = factorial(4);
    int squares = sum_even_squares(6);
    return fact4 + squares;
}
