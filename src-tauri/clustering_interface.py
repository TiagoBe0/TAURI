import sys
import json
import os

# Fix encoding para Windows
if sys.platform == 'win32':
    sys.stdout.reconfigure(encoding='utf-8')
    sys.stderr.reconfigure(encoding='utf-8')

# Agregar el directorio actual al path
sys.path.append(os.path.dirname(os.path.abspath(__file__)))

try:
    from meanshift_cluster import clustering_jerarquico_final
except ImportError as e:
    print(json.dumps({
        'success': False,
        'message': f'Error importando meanshift_cluster: {e}',
        'clusters_found': 0,
        'total_atoms': 0,
        'clusters_info': []
    }))
    sys.exit(1)

def run_clustering(params_json):
    """
    Interfaz para ejecutar el clustering desde Tauri
    """
    try:
        params = json.loads(params_json)
        
        print(f"[INFO] Iniciando clustering con parametros:")
        print(f"   - Archivo: {params['input_file']}")
        print(f"   - Min atomos: {params['min_atoms']}")
        print(f"   - Max iteraciones: {params['max_iterations']}")
        
        # Ejecutar clustering
        result = clustering_jerarquico_final(
            dump_file=params['input_file'],
            output_dir=params['output_dir'],
            min_atoms=params['min_atoms'],
            max_iterations=params['max_iterations'],
            n_clusters_per_level=params.get('n_clusters_per_level'),
            silhouette_threshold=params['silhouette_threshold'],
            davies_bouldin_threshold=params['davies_bouldin_threshold'],
            dispersion_threshold=params.get('dispersion_threshold'),
            quantile=params['quantile'],
            limpiar_intermedios=True
        )
        
        # Preparar respuesta
        response = {
            'success': True,
            'message': 'Clustering completado exitosamente',
            'clusters_found': len(result['clusters_finales']),
            'total_atoms': sum(c['n_atoms'] for c in result['clusters_finales']),
            'clusters_info': [
                {
                    'name': c.get('nombre_final', c.get('nombre', f'cluster_{i}')),
                    'atoms': c['n_atoms'],
                    'level': c['nivel'],
                    'reason': c['razon_final']
                }
                for i, c in enumerate(result['clusters_finales'], 1)
            ]
        }
        
        return json.dumps(response)
        
    except Exception as e:
        import traceback
        error_response = {
            'success': False,
            'message': f'Error durante el clustering: {str(e)}\n{traceback.format_exc()}',
            'clusters_found': 0,
            'total_atoms': 0,
            'clusters_info': []
        }
        return json.dumps(error_response)

if __name__ == "__main__":
    if len(sys.argv) > 1:
        params_json = sys.argv[1]
        result = run_clustering(params_json)
        print(result)
    else:
        error_response = {
            'success': False,
            'message': 'No se proporcionaron parametros',
            'clusters_found': 0,
            'total_atoms': 0,
            'clusters_info': []
        }
        print(json.dumps(error_response))
