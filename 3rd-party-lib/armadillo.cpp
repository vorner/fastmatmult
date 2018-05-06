#include <chrono>
#include <iostream>
#include <armadillo>

int main() {

	for (size_t i = 8; i < 15; i ++) {
		std::cout << "Generating matrices" << std::endl;
		size_t size = 1 << i;
		arma::fmat a = arma::randu<arma::fmat>(size, size);
		arma::fmat b = arma::randu<arma::fmat>(size, size);

		std::cout << "Starting armadillo" << std::endl;
		std::chrono::steady_clock::time_point start = std::chrono::steady_clock::now();

		arma::fmat c = a * b;

		size_t millis = std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::steady_clock::now() - start).count();
		size_t secs = millis / 1000;
		millis %= 1000;

		std::cout << size << ": " << secs << "." << millis << std::endl;
	}
}
